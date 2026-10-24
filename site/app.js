// Refine landing page: download links for the visitor's OS, the latest release info, the
// theme-matched recording, and the one animated example in the hero.

const RELEASES = 'https://github.com/dzhiganov/refine/releases';
const LATEST = `${RELEASES}/latest/download`;
const FILES = {
  mac: { file: 'Refine-macOS.dmg', label: 'Download for macOS' },
  win: { file: 'Refine-Windows-setup.exe', label: 'Download for Windows' },
};
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

// ---- the visitor's OS -------------------------------------------------------------------------

function detectOS() {
  const p = (navigator.userAgentData?.platform || navigator.platform || navigator.userAgent).toLowerCase();
  if (/iphone|ipad|android/.test(navigator.userAgent.toLowerCase())) return null; // phones: show both
  if (p.includes('mac')) return 'mac';
  if (p.includes('win')) return 'win';
  return null;
}

const os = detectOS();
const primary = document.getElementById('primary-download');
const secondary = document.getElementById('secondary-download');

if (os) {
  const other = os === 'mac' ? 'win' : 'mac';
  primary.href = `${LATEST}/${FILES[os].file}`;
  primary.textContent = FILES[os].label;
  secondary.href = '#download';
  secondary.textContent = other === 'mac' ? 'Also for macOS' : 'Also for Windows';
  secondary.hidden = false;
  // In the download section, the visitor's platform goes first; the other one is quieter.
  const list = document.querySelector('.platforms');
  const mine = list.querySelector(`[data-os="${os}"]`);
  list.prepend(mine);
  list.querySelector(`[data-os="${other}"]`).classList.add('other');
} else {
  primary.textContent = 'Download';
  secondary.remove();
}

// ---- latest release (links always point at "latest"; this only adds the details) -------------

const MB = (bytes) => `${Math.round(bytes / 1048576)} MB`;
const dateFmt = new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });

fetch('https://api.github.com/repos/dzhiganov/refine/releases/latest', {
  headers: { Accept: 'application/vnd.github+json' },
})
  .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
  .then((release) => {
    const version = release.tag_name;
    const date = dateFmt.format(new Date(release.published_at));
    document.getElementById('version-line').textContent = `${version} · Free beta`;
    const line = document.getElementById('release-line');
    line.textContent = `${version}, released ${date}. The app is free. `;
    const notes = document.createElement('a');
    notes.href = release.html_url;
    notes.textContent = "What's new";
    line.append(notes);
    for (const asset of release.assets) {
      document.querySelectorAll(`.size[data-asset="${asset.name}"]`).forEach((el) => {
        el.textContent = ` · ${MB(asset.size)}`;
      });
    }
  })
  .catch(() => {
    /* Offline or rate-limited: the links still work, they just don't show the version. */
  });

// ---- the recording: match the theme, play only while visible ---------------------------------
// Sources are set here (not in the HTML) so a light-mode visitor doesn't download the dark video.

const film = document.getElementById('film');
const light = matchMedia('(prefers-color-scheme: light)');
let filmVisible = false;

function setFilm() {
  const t = light.matches ? 'light' : 'dark';
  film.poster = `/media/poster-${t}.webp`;
  film.innerHTML = `<source src="/media/demo-${t}.webm" type="video/webm"><source src="/media/demo-${t}.mp4" type="video/mp4">`;
  film.load();
  if (filmVisible && !reducedMotion) film.play().catch(() => {});
}

setFilm();
light.addEventListener('change', setFilm);

if (reducedMotion) {
  film.controls = true;
} else {
  new IntersectionObserver(
    ([e]) => {
      filmVisible = e.isIntersecting;
      if (filmVisible) film.play().catch(() => {});
      else film.pause();
    },
    { threshold: 0.25 },
  ).observe(film);
  // Click (or Enter/Space when focused) pauses and resumes.
  film.tabIndex = 0;
  const toggle = () => (film.paused ? film.play().catch(() => {}) : film.pause());
  film.addEventListener('click', toggle);
  film.addEventListener('keydown', (e) => {
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      toggle();
    }
  });
}

// ---- the animated example ------------------------------------------------------------------------
// Real corrections from Qwen 3.6 (LM Studio). The explanations are the model's own words; the
// highlighted fragments are trimmed to the words that change.

const EXAMPLES = [
  {
    tone: 'Formal email',
    parts: [
      'Dear Mr. Weber, ',
      ['thx', 'thank you', "Use full words in formal emails. 'Thx' is a chat abbreviation."],
      ' for the quick reply. Yesterday I ',
      ['have sent', 'sent', "Use the simple past (sent) with specific past times like 'yesterday'."],
      ' the contract, ',
      ['pls', 'please', "Use full words in formal emails. 'Pls' is a chat abbreviation."],
      ' let me know ',
      ['until', 'by', "Use 'by' for deadlines (before or on that day). 'Until' means the action continues up to that time."],
      ' Friday if everything is fine.',
    ],
  },
  {
    tone: 'Slack',
    parts: [
      'Hallo zusammen, ich habe gestern mit ',
      ['der Kunde', 'dem Kunden', "The noun 'Kunde' is masculine. After the preposition 'mit', you must use the dative case. The dative form is 'dem Kunden'."],
      ' gesprochen und er hat gesagt, dass er ',
      ['braucht mehr Zeit', 'mehr Zeit braucht', "In a subordinate clause starting with 'dass', the conjugated verb must go to the very end."],
      ' für die Tests.',
    ],
  },
];

const textEl = document.getElementById('demo-text');
const whyEl = document.getElementById('demo-why');
const toneEl = document.getElementById('demo-tone');
const countEl = document.getElementById('demo-count');
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

function render(example) {
  toneEl.textContent = example.tone;
  countEl.textContent = '';
  textEl.replaceChildren();
  const fixes = [];
  for (const part of example.parts) {
    if (typeof part === 'string') {
      textEl.append(part);
      continue;
    }
    const [was, now, why] = part;
    const span = document.createElement('span');
    span.className = 'fix';
    span.innerHTML = '<span class="was"></span><span class="now"></span>';
    span.querySelector('.was').textContent = was;
    span.querySelector('.now').textContent = now;
    textEl.append(span);
    fixes.push({ span, was, now, why });
  }
  return fixes;
}

function explain(fix) {
  whyEl.classList.remove('hidden');
  whyEl.innerHTML = '<strong></strong> ';
  whyEl.querySelector('strong').textContent = `${fix.was} → ${fix.now}.`;
  whyEl.append(fix.why);
}

// Pause the loop while the hero is off screen, the tab is hidden, or the visitor paused it.
let visible = true;
let paused = false;
new IntersectionObserver(([e]) => (visible = e.isIntersecting)).observe(textEl);
const whenVisible = async () => {
  while (!visible || document.hidden || paused) await wait(300);
};
const pauseButton = document.getElementById('demo-pause');
pauseButton.addEventListener('click', () => {
  paused = !paused;
  pauseButton.textContent = paused ? 'Play' : 'Pause';
});

async function play() {
  for (let i = 0; ; i = (i + 1) % EXAMPLES.length) {
    const fixes = render(EXAMPLES[i]);
    whyEl.classList.add('hidden');
    textEl.classList.remove('fading');
    await wait(1100);
    for (const [n, fix] of fixes.entries()) {
      await whenVisible();
      fix.span.classList.add('marked');
      explain(fix);
      await wait(650);
      fix.span.classList.add('struck');
      countEl.textContent = `${n + 1} ${n === 0 ? 'fix' : 'fixes'}`;
      // Longer explanations stay up longer.
      await wait(1300 + fix.why.length * 12);
    }
    // The clean result, ready to copy.
    whyEl.classList.add('hidden');
    fixes.forEach((f) => f.span.classList.add('clean'));
    await wait(2800);
    await whenVisible();
    textEl.classList.add('fading');
    await wait(450);
  }
}

if (reducedMotion) {
  const fixes = render(EXAMPLES[0]);
  fixes.forEach((f) => f.span.classList.add('marked', 'struck'));
  countEl.textContent = `${fixes.length} fixes`;
  explain(fixes[3]);
} else {
  pauseButton.hidden = false;
  play();
}
