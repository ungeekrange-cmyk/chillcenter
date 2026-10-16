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

test('series synopsis, counts and rating share the header copy; TMDB and edit controls are separate', async () => {
  const f = seriesFixture([episode('s1', 1, 1)]);
  await f.execute('openSeries("Fixture")');
  const header = f.get('series-common');
  assert.equal(header.className, 'series-overview');
  assert.ok(hasClass(header.children[0], 'series-poster'));
  const copy = header.children[1];
  assert.equal(copy.className, 'series-overview-copy');
  assert.ok(copy.children.some(item => item.textContent === 'Informations communes à compléter'));
  assert.ok(copy.children.some(item => item.textContent === '1 saisons · 1 épisodes · 1 épisodes non vus'));
  assert.ok(copy.children.some(item => item.className === 'rating-stars' && item.children.length === 5));
  assert.equal(f.get('collection-counts').hidden, true);
  const tools = f.get('series-tools');
  assert.ok(tools.children.some(item => item.tagName === 'SECTION'));
  assert.ok(tools.children.some(item => item.tagName === 'FORM'));
  assert.equal(header.children.some(item => item.tagName === 'FORM'), false);
});
