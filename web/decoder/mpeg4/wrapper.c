// Minimal MPEG-4 Part 2 (Xvid/DivX) decoder bridge over FFmpeg's libavcodec (ADR-0014).
// Whole interface: open / send / receive / flush / reset / close, plus plane accessors.
// Packets are handed in by JavaScript; frames are read back plane by plane and copied there.
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <emscripten.h>
#include <libavcodec/avcodec.h>
#include <libavutil/error.h>

static AVCodecContext *ctx = NULL;
static AVPacket *pkt = NULL;
static AVFrame *frame = NULL;

EMSCRIPTEN_KEEPALIVE int cc_open(void) {
  const AVCodec *codec = avcodec_find_decoder(AV_CODEC_ID_MPEG4);
  if (!codec) return AVERROR_DECODER_NOT_FOUND;
  ctx = avcodec_alloc_context3(codec);
  pkt = av_packet_alloc();
  frame = av_frame_alloc();
  if (!ctx || !pkt || !frame) return AVERROR(ENOMEM);
  ctx->thread_count = 1;           // single thread: no SharedArrayBuffer / COOP-COEP needed
  ctx->flags2 |= AV_CODEC_FLAG2_FAST;
  return avcodec_open2(ctx, codec, NULL);
}

// Input buffers come from cc_alloc so that they carry the zeroed padding libavcodec requires.
EMSCRIPTEN_KEEPALIVE uint8_t *cc_alloc(int size) {
  uint8_t *p = (uint8_t *)malloc((size_t)size + AV_INPUT_BUFFER_PADDING_SIZE);
  if (p) memset(p + size, 0, AV_INPUT_BUFFER_PADDING_SIZE);
  return p;
}
EMSCRIPTEN_KEEPALIVE void cc_free(uint8_t *p) { free(p); }

// pts travels as a double (exact up to 2^53) to avoid BigInt at the JS boundary.
EMSCRIPTEN_KEEPALIVE int cc_send(uint8_t *data, int size, double pts) {
  pkt->data = data;
  pkt->size = size;
  pkt->pts = (int64_t)pts;
  pkt->dts = AV_NOPTS_VALUE;
  return avcodec_send_packet(ctx, pkt);
}
EMSCRIPTEN_KEEPALIVE int cc_flush_send(void) { return avcodec_send_packet(ctx, NULL); }

// 1 = a frame is ready (valid until the next call), 0 = need more input / drained, <0 = error.
EMSCRIPTEN_KEEPALIVE int cc_receive(void) {
  int r = avcodec_receive_frame(ctx, frame);
  if (r == 0) return 1;
  if (r == AVERROR(EAGAIN) || r == AVERROR_EOF) return 0;
  return r;
}
EMSCRIPTEN_KEEPALIVE void cc_reset(void) { if (ctx) avcodec_flush_buffers(ctx); }
EMSCRIPTEN_KEEPALIVE void cc_close(void) {
  av_frame_free(&frame);
  av_packet_free(&pkt);
  avcodec_free_context(&ctx);
}

EMSCRIPTEN_KEEPALIVE int cc_width(void) { return frame->width; }
EMSCRIPTEN_KEEPALIVE int cc_height(void) { return frame->height; }
EMSCRIPTEN_KEEPALIVE int cc_format(void) { return frame->format; }  // AV_PIX_FMT_YUV420P = 0, YUVJ420P = 12
EMSCRIPTEN_KEEPALIVE double cc_pts(void) {
  return frame->pts != AV_NOPTS_VALUE ? (double)frame->pts : (double)frame->best_effort_timestamp;
}
EMSCRIPTEN_KEEPALIVE const uint8_t *cc_plane(int i) { return frame->data[i]; }
EMSCRIPTEN_KEEPALIVE int cc_stride(int i) { return frame->linesize[i]; }
