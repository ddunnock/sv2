/*
 * Model Workbench mockup — interaction script (no dependencies).
 * Everything the screens can show is already in the HTML; this script only
 * toggles visibility and selection classes. Hooks:
 *   [data-action]         clickable control (toggleNav, toggleSpec, showFiles, showModel,
 *                         showSpec, showSrc, pick.<ElementName>)
 *   [data-when]           region shown only while a state flag is true
 *                         (navOpen, specOpen, specClosed, isSpec, isSrc)
 *   [data-explorer]       navigator tree variant: "files" | "elements"
 *   [data-element-panel]  sidebar content for one selected element
 */
(function () {
  var root = document.querySelector('.wb-root');
  if (!root) return;
  function all(sel) { return Array.prototype.slice.call(root.querySelectorAll(sel)); }
  function shown(sel) { var e = root.querySelector(sel); return !!e && !e.hidden; }
  var panel = all('[data-element-panel]').filter(function (e) { return !e.hidden; })[0];
  var S = {
    nav: shown('[data-when="navOpen"]'),
    spec: shown('[data-when="specOpen"]'),
    mode: 'files',
    sel: panel ? panel.getAttribute('data-element-panel') : null,
    tab: 'spec',
    dark: false
  };
  function when(name, on) { all('[data-when="' + name + '"]').forEach(function (e) { e.hidden = !on; }); }
  function pressed(action, on, attr) {
    all('[data-action="' + action + '"]').forEach(function (b) {
      b.classList.toggle('on', on);
      b.setAttribute(attr, on ? 'true' : 'false');
    });
  }
  function apply() {
    if (root.querySelector('[data-when="navOpen"]')) when('navOpen', S.nav);
    if (root.querySelector('[data-when="specOpen"]')) { when('specOpen', S.spec); when('specClosed', !S.spec); }
    when('isSpec', S.tab === 'spec');
    when('isSrc', S.tab === 'src');
    all('[aria-label="Toggle navigator"], [aria-label="Explorer"]').forEach(function (b) { b.classList.toggle('on', S.nav); });
    all('[aria-label="Toggle specification sidebar"]').forEach(function (b) { b.classList.toggle('on', S.spec); });
    if (root.querySelector('[data-explorer]')) {
      all('[data-explorer]').forEach(function (e) { e.hidden = e.getAttribute('data-explorer') !== S.mode; });
      pressed('showFiles', S.mode === 'files', 'aria-pressed');
      pressed('showModel', S.mode === 'elements', 'aria-pressed');
      var f = root.querySelector('[data-region="navigator"] input');
      if (f) f.placeholder = S.mode === 'files' ? 'Filter files' : 'Filter elements (name, kind, type)';
    }
    if (S.sel) {
      all('[data-element-panel]').forEach(function (e) { e.hidden = e.getAttribute('data-element-panel') !== S.sel; });
      all('[data-action^="pick."]').forEach(function (b) {
        var on = b.getAttribute('data-action').slice(5) === S.sel;
        b.classList.toggle(b.classList.contains('node') ? 'sel' : 'on', on);
      });
      var n = root.querySelector('[data-slot="selname"]');
      if (n) n.textContent = 'Selected: ' + S.sel;
      pressed('showSpec', S.tab === 'spec', 'aria-selected');
      pressed('showSrc', S.tab === 'src', 'aria-selected');
    }
    root.classList.toggle('t-dark', S.dark);
    root.classList.toggle('t-light', !S.dark);
    var t = document.getElementById('theme-toggle');
    if (t) t.textContent = S.dark ? 'Light theme' : 'Dark theme';
  }
  root.addEventListener('click', function (ev) {
    var a = ev.target.closest('a[href="#"]');
    if (a) ev.preventDefault();
    var b = ev.target.closest('[data-action]');
    if (!b) return;
    var act = b.getAttribute('data-action');
    if (act === 'toggleNav') S.nav = !S.nav;
    else if (act === 'toggleSpec') S.spec = !S.spec;
    else if (act === 'showFiles') S.mode = 'files';
    else if (act === 'showModel') S.mode = 'elements';
    else if (act === 'showSpec') S.tab = 'spec';
    else if (act === 'showSrc') S.tab = 'src';
    else if (act.indexOf('pick.') === 0) S.sel = act.slice(5);
    apply();
  });
  var tt = document.getElementById('theme-toggle');
  if (tt) tt.addEventListener('click', function () { S.dark = !S.dark; apply(); });
  // Deep-link state for reviewers and agents: ?explorer=elements&select=Heater&tab=source&nav=0&spec=0&theme=dark
  var q = new URLSearchParams(location.search);
  if (q.get('explorer') === 'elements') S.mode = 'elements';
  if (q.get('select') && root.querySelector('[data-element-panel="' + q.get('select') + '"]')) S.sel = q.get('select');
  if (q.get('tab') === 'source') S.tab = 'src';
  if (q.get('nav') === '0') S.nav = false;
  if (q.get('nav') === '1') S.nav = true;
  if (q.get('spec') === '0') S.spec = false;
  if (q.get('spec') === '1') S.spec = true;
  if (q.get('theme') === 'dark') S.dark = true;
  window.workbenchState = S;
  apply();
})();
