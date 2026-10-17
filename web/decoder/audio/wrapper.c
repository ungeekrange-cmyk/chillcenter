// Audio transcoder for the web client (ADR-0020): AC-3 / E-AC-3 / DTS / TrueHD / MP3 access units in, AAC-LC access units out.
// Browsers do not decode these codecs through Media Source Extensions (the app browser does not; Safari does), so the
// client converts them locally: decode (FFmpeg libavcodec), mix down and resample (libswresample), encode (native AAC encoder).
// The server never transcodes anything. LGPL only: no external library, no GPL component.
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <emscripten.h>
#include <libavcodec/avcodec.h>
#include <libavutil/audio_fifo.h>
#include <libavutil/channel_layout.h>
#include <libavutil/error.h>
#include <libavutil/frame.h>
#include <libavutil/mem.h>
#include <libavutil/opt.h>
#include <libswresample/swresample.h>

#define AAC_FRAME 1024

enum { CC_AC3 = 0, CC_EAC3 = 1, CC_DTS = 2, CC_TRUEHD = 3, CC_MLP = 4, CC_MP3 = 5 };

static AVCodecContext *dec = NULL, *enc = NULL;
static SwrContext *swr = NULL;
static AVAudioFifo *fifo = NULL;
static AVPacket *in_pkt = NULL, *out_pkt = NULL;
static AVFrame *dec_frame = NULL, *conv_frame = NULL, *enc_frame = NULL;
static int out_rate = 48000, out_channels = 2, bitrate = 192000;
static int src_rate = 0, src_channels = 0, errors = 0;
static uint8_t *queue = NULL;       // AAC packets waiting for JS: [u32 size][bytes]...
static size_t queue_len = 0, queue_cap = 0, queue_read = 0;
static uint8_t current[16384];
static int current_len = 0;

static enum AVCodecID codec_id(int c) {
  switch (c) {
    case CC_AC3: return AV_CODEC_ID_AC3;
    case CC_EAC3: return AV_CODEC_ID_EAC3;
    case CC_DTS: return AV_CODEC_ID_DTS;
    case CC_TRUEHD: return AV_CODEC_ID_TRUEHD;
    case CC_MLP: return AV_CODEC_ID_MLP;
    case CC_MP3: return AV_CODEC_ID_MP3;
    default: return AV_CODEC_ID_NONE;
  }
}

static int push_packet(const uint8_t *data, int size) {
  if (queue_read == queue_len) { queue_len = queue_read = 0; }
  if (queue_len + 4 + (size_t)size > queue_cap) {
    size_t cap = queue_cap ? queue_cap : 65536;
    while (queue_len + 4 + (size_t)size > cap) cap *= 2;
    uint8_t *grown = realloc(queue, cap);
    if (!grown) return AVERROR(ENOMEM);
    queue = grown; queue_cap = cap;
  }
  memcpy(queue + queue_len, &size, 4); memcpy(queue + queue_len + 4, data, (size_t)size);
  queue_len += 4 + (size_t)size;
  return 0;
}

static int encode_frame(AVFrame *frame) {   // frame == NULL flushes the encoder
  int r = avcodec_send_frame(enc, frame);
  if (r < 0) return r;
  for (;;) {
    r = avcodec_receive_packet(enc, out_pkt);
    if (r == AVERROR(EAGAIN) || r == AVERROR_EOF) return 0;
    if (r < 0) return r;
    r = push_packet(out_pkt->data, out_pkt->size);
    av_packet_unref(out_pkt);
    if (r < 0) return r;
  }
}

static int drain_fifo(int pad) {            // full AAC frames out of the FIFO; pad = silence-fill the last partial frame
  while (av_audio_fifo_size(fifo) >= AAC_FRAME || (pad && av_audio_fifo_size(fifo) > 0)) {
    int n = av_audio_fifo_size(fifo); if (n > AAC_FRAME) n = AAC_FRAME;
    av_frame_make_writable(enc_frame);
    enc_frame->nb_samples = AAC_FRAME;
    if (av_audio_fifo_read(fifo, (void **)enc_frame->data, n) < n) return AVERROR(EIO);
    for (int ch = 0; ch < out_channels; ch++) if (n < AAC_FRAME) memset(enc_frame->data[ch] + n * sizeof(float), 0, (size_t)(AAC_FRAME - n) * sizeof(float));
    int r = encode_frame(enc_frame);
    if (r < 0) return r;
  }
  return 0;
}

static int init_swr(AVFrame *f) {
  AVChannelLayout out_layout; av_channel_layout_default(&out_layout, out_channels);
  swr_free(&swr);
  int r = swr_alloc_set_opts2(&swr, &out_layout, AV_SAMPLE_FMT_FLTP, out_rate, &f->ch_layout, f->format, f->sample_rate, 0, NULL);
  if (r < 0) return r;
  src_rate = f->sample_rate; src_channels = f->ch_layout.nb_channels;
  return swr_init(swr);
}

static int convert(AVFrame *f) {            // decoded frame -> resampled/mixed float planar -> FIFO
  if (!swr && init_swr(f) < 0) return AVERROR(EINVAL);
  int cap = swr_get_out_samples(swr, f->nb_samples) + 64;
  av_frame_unref(conv_frame);
  conv_frame->nb_samples = cap; conv_frame->format = AV_SAMPLE_FMT_FLTP; conv_frame->sample_rate = out_rate;
  av_channel_layout_default(&conv_frame->ch_layout, out_channels);
  int r = av_frame_get_buffer(conv_frame, 0);
  if (r < 0) return r;
  int got = swr_convert(swr, conv_frame->extended_data, cap, (const uint8_t **)f->extended_data, f->nb_samples);
  if (got < 0) return got;
  // Mix-down (libswresample's standard matrix: centre and surrounds at -3 dB, LFE dropped) can sum above full scale: smooth knee from 0.9
  // up to 1.0 instead of letting the encoder or the output clip hard.
  for (int ch = 0; ch < out_channels; ch++) {
    float *x = (float *)conv_frame->extended_data[ch];
    for (int i = 0; i < got; i++) {
      float v = x[i], a = fabsf(v);
      if (a > 0.9f) x[i] = copysignf(0.9f + 0.1f * tanhf((a - 0.9f) / 0.1f), v);
    }
  }
  if (got > 0 && av_audio_fifo_write(fifo, (void **)conv_frame->extended_data, got) < got) return AVERROR(ENOMEM);
  return drain_fifo(0);
}

static int receive_decoded(void) {
  for (;;) {
    int r = avcodec_receive_frame(dec, dec_frame);
    if (r == AVERROR(EAGAIN) || r == AVERROR_EOF) return 0;
    if (r < 0) return r;
    r = convert(dec_frame);
    av_frame_unref(dec_frame);
    if (r < 0) return r;
  }
}

static void teardown(void) {
  avcodec_free_context(&dec); avcodec_free_context(&enc);
  swr_free(&swr);
  if (fifo) { av_audio_fifo_free(fifo); fifo = NULL; }
  av_packet_free(&in_pkt); av_packet_free(&out_pkt);
  av_frame_free(&dec_frame); av_frame_free(&conv_frame); av_frame_free(&enc_frame);
  queue_len = queue_read = 0; current_len = 0; src_rate = src_channels = 0;
}

EMSCRIPTEN_KEEPALIVE int cc_a_open(int codec, int rate, int channels, int kbps) {
  teardown();
  out_rate = rate; out_channels = channels; bitrate = kbps * 1000;
  const AVCodec *d = avcodec_find_decoder(codec_id(codec)), *e = avcodec_find_encoder(AV_CODEC_ID_AAC);
  if (!d || !e) return AVERROR_DECODER_NOT_FOUND;
  dec = avcodec_alloc_context3(d); enc = avcodec_alloc_context3(e);
  in_pkt = av_packet_alloc(); out_pkt = av_packet_alloc();
  dec_frame = av_frame_alloc(); conv_frame = av_frame_alloc(); enc_frame = av_frame_alloc();
  if (!dec || !enc || !in_pkt || !out_pkt || !dec_frame || !conv_frame || !enc_frame) return AVERROR(ENOMEM);
  dec->thread_count = 1; enc->thread_count = 1;
  int r = avcodec_open2(dec, d, NULL);
  if (r < 0) return r;
  enc->sample_rate = out_rate; enc->sample_fmt = AV_SAMPLE_FMT_FLTP; enc->bit_rate = bitrate;
  enc->time_base = (AVRational){1, out_rate};
  av_channel_layout_default(&enc->ch_layout, out_channels);
  enc->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;          // AudioSpecificConfig in extradata, not in every frame
  r = avcodec_open2(enc, e, NULL);
  if (r < 0) return r;
  fifo = av_audio_fifo_alloc(AV_SAMPLE_FMT_FLTP, out_channels, AAC_FRAME * 4);
  enc_frame->format = AV_SAMPLE_FMT_FLTP; enc_frame->sample_rate = out_rate; enc_frame->nb_samples = AAC_FRAME;
  av_channel_layout_default(&enc_frame->ch_layout, out_channels);
  if (!fifo || av_frame_get_buffer(enc_frame, 0) < 0) return AVERROR(ENOMEM);
  return 0;
}

// Input buffers carry the zeroed padding libavcodec requires.
EMSCRIPTEN_KEEPALIVE uint8_t *cc_a_alloc(int size) {
  uint8_t *p = (uint8_t *)malloc((size_t)size + AV_INPUT_BUFFER_PADDING_SIZE);
  if (p) memset(p + size, 0, AV_INPUT_BUFFER_PADDING_SIZE);
  return p;
}
EMSCRIPTEN_KEEPALIVE void cc_a_free(uint8_t *p) { free(p); }

EMSCRIPTEN_KEEPALIVE int cc_a_send(uint8_t *data, int size) {
  in_pkt->data = data; in_pkt->size = size;
  int r = avcodec_send_packet(dec, in_pkt);
  if (r < 0) { errors++; return 0; }                 // a damaged packet is skipped: a short glitch, never a stop
  r = receive_decoded();
  if (r < 0) { errors++; return 0; }
  return 0;
}

// End of stream: drain the decoder, flush the resampler, pad the last partial frame with silence, flush the encoder.
EMSCRIPTEN_KEEPALIVE int cc_a_flush(void) {
  if (!dec) return 0;
  avcodec_send_packet(dec, NULL); receive_decoded();
  if (swr) {
    int cap = swr_get_out_samples(swr, 0) + 64;
    av_frame_unref(conv_frame);
    conv_frame->nb_samples = cap; conv_frame->format = AV_SAMPLE_FMT_FLTP; conv_frame->sample_rate = out_rate;
    av_channel_layout_default(&conv_frame->ch_layout, out_channels);
    if (av_frame_get_buffer(conv_frame, 0) == 0) { int got = swr_convert(swr, conv_frame->extended_data, cap, NULL, 0); if (got > 0) av_audio_fifo_write(fifo, (void **)conv_frame->extended_data, got); }
  }
  drain_fifo(1);
  return encode_frame(NULL);
}

// Seek: forget everything in flight, keep the codecs configured. A new encoder is opened (the native AAC encoder has no reset).
EMSCRIPTEN_KEEPALIVE int cc_a_reset(void) {
  if (!dec) return 0;
  avcodec_flush_buffers(dec);
  swr_free(&swr);
  av_audio_fifo_reset(fifo);
  queue_len = queue_read = 0; current_len = 0;
  const AVCodec *e = avcodec_find_encoder(AV_CODEC_ID_AAC);
  avcodec_free_context(&enc);
  enc = avcodec_alloc_context3(e);
  if (!enc) return AVERROR(ENOMEM);
  enc->thread_count = 1; enc->sample_rate = out_rate; enc->sample_fmt = AV_SAMPLE_FMT_FLTP; enc->bit_rate = bitrate;
  enc->time_base = (AVRational){1, out_rate};
  av_channel_layout_default(&enc->ch_layout, out_channels);
  enc->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
  return avcodec_open2(enc, e, NULL);
}

// Next finished AAC access unit: returns its size (0 = none yet, <0 = too large); bytes at cc_a_packet().
EMSCRIPTEN_KEEPALIVE int cc_a_next(void) {
  if (queue_read >= queue_len) return 0;
  int size; memcpy(&size, queue + queue_read, 4);
  if (size > (int)sizeof(current)) return -1;
  memcpy(current, queue + queue_read + 4, (size_t)size);
  queue_read += 4 + (size_t)size; current_len = size;
  return size;
}
EMSCRIPTEN_KEEPALIVE const uint8_t *cc_a_packet(void) { return current; }
EMSCRIPTEN_KEEPALIVE const uint8_t *cc_a_asc(void) { return enc ? enc->extradata : NULL; }
EMSCRIPTEN_KEEPALIVE int cc_a_asc_size(void) { return enc ? enc->extradata_size : 0; }
EMSCRIPTEN_KEEPALIVE int cc_a_frame_size(void) { return enc ? enc->frame_size : 0; }
EMSCRIPTEN_KEEPALIVE int cc_a_initial_padding(void) { return enc ? enc->initial_padding : 0; }
EMSCRIPTEN_KEEPALIVE int cc_a_src_rate(void) { return src_rate; }
EMSCRIPTEN_KEEPALIVE int cc_a_src_channels(void) { return src_channels; }
EMSCRIPTEN_KEEPALIVE int cc_a_errors(void) { return errors; }
EMSCRIPTEN_KEEPALIVE void cc_a_close(void) { teardown(); free(queue); queue = NULL; queue_cap = 0; }
