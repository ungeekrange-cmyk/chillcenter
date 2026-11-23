const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./library-harness.js');

const episode = (id, season, number, watched = false) => ({
  id, season, episode: number, watched, title: id, series: 'Fixture',
  category: 'serie', available: true, poster: false,
});
const hasClass = (element, name) => element.className.split(/\s+/).includes(name);

function seriesFixture(episodes) {
  const f = fixture(async command => {
    assert.equal(command.action, 'series');
    return {
      name: command.name, episodes, metadata: { name: command.name },
      revision: 1, counters: { unwatched: episodes.filter(item => !item.watched).length },
    };
  });
  // Only this test needs native <details> selection; do not alter other harness users.
  f.get('collection-members').querySelectorAll = selector => {
    const sections = f.get('collection-members').children.filter(item => hasClass(item, 'series-season'));
    return selector.includes('[open]') ? sections.filter(item => item.open) : sections;
  };
  f.sections = () => f.get('collection-members').children;
  f.openSeasons = () => f.sections().filter(section => section.open).map(section => section.dataset.season);
  return f;
}

test('opens the first chronologically ordered season with unseen episodes, not the first returned item', async () => {
  const f = seriesFixture([
    episode('s3', 3, 1), episode('s1', 1, 1, true),
    episode('s2-seen', 2, 1, true), episode('s2-new', 2, 2),
  ]);
  await f.execute('openSeries("Fixture")');
  assert.deepEqual(f.sections().map(section => section.dataset.season), ['1', '2', '3']);
  assert.deepEqual(f.openSeasons(), ['2']);
  assert.equal(f.get('collection-counts').textContent, '3 saisons · 4 épisodes · 2 épisodes non vus');
});

test('all-seen series opens the first season; empty series has no phantom season', async () => {
  const seen = seriesFixture([episode('s2', 2, 1, true), episode('s1', 1, 1, true)]);
  await seen.execute('openSeries("Seen")');
  assert.deepEqual(seen.openSeasons(), ['1']);
  const empty = seriesFixture([]);
  await empty.execute('openSeries("Empty")');
  assert.deepEqual(empty.sections(), []);
});

test('unknown seasons sort last but can open if they contain the only unseen episode', async () => {
  const f = seriesFixture([episode('unknown', null, null), episode('known', 1, 1, true)]);
  await f.execute('openSeries("Fixture")');
  assert.deepEqual(f.sections().map(section => section.dataset.season), ['1', 'null']);
  assert.deepEqual(f.openSeasons(), ['null']);
});

test('refresh preserves manually chosen seasons, including an intentionally fully collapsed series', async () => {
  const f = seriesFixture([episode('s1', 1, 1), episode('s2', 2, 1)]);
  await f.execute('openSeries("Fixture")');
  f.sections()[0].open = false;
  f.sections()[1].open = true;
  await f.execute('openSeries("Fixture")');
  assert.deepEqual(f.openSeasons(), ['2']);
  for (const section of f.sections()) section.open = false;
  await f.execute('openSeries("Fixture")');
  assert.deepEqual(f.openSeasons(), []);
  f.sandbox.shellSection = 'catalog';
  await f.execute('openSeries("Fixture")');
  assert.deepEqual(f.openSeasons(), ['1'], 'opening anew from the catalogue reapplies the automatic choice');
});

test('each season owns an episode grid, sorted cards and the original series playback context', async () => {
  const f = seriesFixture([episode('second', 1, 2), episode('first', 1, 1)]);
  const calls = [];
  f.sandbox.window.chillPlay = (id, context) => calls.push({ mediaId: id, kind: context.kind, collectionId: context.id });
  await f.execute('openSeries("opaque-key")');
  const section = f.sections()[0];
  const grid = section.children.find(item => hasClass(item, 'series-episodes'));
  assert.ok(grid, 'episodes need their own grid inside native details content');
  assert.ok(hasClass(grid, 'collection-grid'));
  assert.deepEqual(grid.children.map(card => card.dataset.mediaId), ['first', 'second']);
  assert.equal(section.children.filter(item => hasClass(item, 'collection-media-card')).length, 0);
  for (const card of grid.children) assert.ok(hasClass(card, 'collection-media-card'));
  const actions = grid.children[0].children[1].children[2];
  await actions.children[0].onclick();
  assert.deepEqual(calls, [{ mediaId: 'first', kind: 'series', collectionId: 'opaque-key' }]);
});

test('series counts are in the header and edit controls for the series and seasons are collapsed', async () => {
  const f = seriesFixture([episode('s1', 1, 1)]);
  await f.execute('openSeries("Fixture")');
  const header = f.get('series-common');
  assert.equal(header.className, 'series-overview');
  assert.ok(hasClass(header.children[0], 'series-poster'));
  const copy = header.children[1];
  assert.equal(copy.className, 'series-overview-copy');
  assert.equal(copy.children.length, 1, 'omit absent metadata and placeholder copy');
  assert.ok(copy.children.some(item => item.textContent === '1 saisons · 1 épisodes · 1 épisodes non vus'));
  assert.equal(copy.children.some(item => item.className === 'rating-stars'), false);
  assert.equal(f.get('collection-counts').hidden, true);
  const tools = f.get('series-tools');
  assert.equal(tools.hidden, true);
  assert.ok(tools.children.some(item => item.className === 'rating-stars' && item.children.length === 5));
  const seasonTools = tools.children.find(item => hasClass(item, 'season-provider'));
  assert.equal(seasonTools.tagName, 'DETAILS');
  assert.ok(!seasonTools.open);
  assert.ok(tools.children.some(item => item.tagName === 'SECTION'));
  assert.ok(tools.children.some(item => item.tagName === 'FORM'));
  assert.equal(header.children.some(item => item.tagName === 'FORM'), false);
});
test('Home can explicitly open a started season0 even when another season has unseen episodes',async()=>{
 const f=seriesFixture([episode('s0',0,1),episode('s1',1,1)]);await f.execute('openSeries("Fixture",0)');assert.deepEqual(f.openSeasons(),['0']);
});
test('Home can explicitly open an unknown started season while a known season is untouched',async()=>{
 const f=seriesFixture([episode('unknown',null,1),episode('known',1,1)]);await f.execute('openSeries("Fixture",null)');assert.deepEqual(f.openSeasons(),['null']);
});


test('the series button names and plays the same episode, and a catalogue action launches it directly', async () => {
  const f = seriesFixture([episode('unseen', 2, 1), episode('resume', 2, 2)]);
  const launches = [];
  f.sandbox.window.seriesPlaybackTarget = episodes => episodes.find(item => item.id === 'resume');
  f.sandbox.window.chillPlay = (id, context) => launches.push([id, context.kind, context.id]);
  await f.execute('openSeries("Fixture")');
  const controls = f.get('series-actions').children[0];
  assert.equal(controls.children[0].textContent, 'Lire');
  assert.equal(controls.children[1].textContent, 'Saison 2 · Épisode 2 · resume');
  await controls.children[0].onclick();
  f.sandbox.shellSection = 'catalog';
  await f.execute('directPlaybackButton({title:"Fixture",available:true},{seriesKey:"Fixture"}).onclick()');
  assert.deepEqual(launches, [['resume', 'series', 'Fixture'], ['resume', 'series', 'Fixture']]);
  assert.equal(f.sandbox.shellSection, 'catalog');
});
