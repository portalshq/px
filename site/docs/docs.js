(() => {
  const pages = window.PX_DOCS;
  const groups = [
    ['Start here', [['authored/introduction','Introduction'],['authored/installation','Installation & quick start'],['authored/usage','Using PX']]],
    ['Learn the concepts', [['authored/primitives','Core primitives'],['authored/primitives-examples','Primitives in practice'],['authored/design-principles','Design principles']]],
    ['Using PX', [['authored/usage/reference','CLI examples'],['authored/usage/narrative-resource-addressing','Resource addressing'],['authored/usage/portability','Portability'],['authored/usage/resolution','Resolution'],['authored/usage/world-building','World-building'],['authored/usage/pro-tips','Practical tips'],['authored/usage/repository-layout-reference','Repository layout'],['authored/usage/cli-command-reference','Command reference'],['authored/usage/design-principles','Design principles']]],
    ['Workflows (for agents)', [['authored/workflows/resolve-workflow','Resolve an entity'],['authored/workflows/update-workflow','Update an entity'],['authored/workflows/continuity','Continuity'],['authored/workflows/provenance','Provenance'],['authored/workflows/revision-branches','Revision branches'],['authored/workflows/promotion','Promotion'],['authored/workflows/repository-stewardship','Repository stewardship'],['authored/workflows/target-branch','Target branch']]],
    ['Reference', [['authored/mcp/overview','MCP overview'],['authored/mcp/install','Connect an agent'],['generated/cli','CLI reference'],['generated/mcp','MCP reference'],['authored/character-sheet-spec','Character sheet spec']]]
  ];
  const labels = Object.fromEntries(groups.flatMap(([, items]) => items));
  const filename = location.pathname.split('/').pop().replace(/\.html$/, '');
  let key = !filename || filename === 'index' ? 'authored/introduction' : filename.replaceAll('--', '/');
  if (!pages[key]) key = 'authored/introduction';
  const page = pages[key];
  const title = labels[key] || page.title;
  const group = groups.find(([, items]) => items.some(([id]) => id === key));
  const nav = document.querySelector('#nav');
  const href = id => id === 'authored/introduction' ? './' : id.replaceAll('/', '--') + '.html';
  groups.forEach(([name, items]) => {
    const section = document.createElement('section'); section.className = 'nav-group';
    const heading = document.createElement('p'); heading.className = 'nav-heading'; heading.textContent = name; section.append(heading);
    items.forEach(([id, label]) => { if (!pages[id]) return; const a = document.createElement('a'); a.className = 'nav-link' + (id === key ? ' active' : ''); a.href = href(id); a.textContent = label; if (id === key) a.setAttribute('aria-current','page'); section.append(a); }); nav.append(section);
  });
  document.title = title + ' — PX Documentation';
  const intro = key === 'authored/introduction';
  const article = document.querySelector('#article');
  const capitalCase = text => text.replace(/\b[a-z]/g, letter => letter.toUpperCase());
  article.innerHTML = `<div class="breadcrumb">Documentation <span>/</span> ${capitalCase(group ? group[0] : 'Reference')} <span>/</span> ${capitalCase(title)}</div><div class="article-hero"><div class="hero-top"><a class="markdown-link" href="${key === 'authored/introduction' ? 'index' : key.replaceAll('/', '--')}.md">View Markdown ↗</a></div><h1>${intro ? 'Introduction' : title}</h1><p class="lead">${intro ? 'The agentic toolkit for persistent multimedia projects.' : ''}</p></div>${intro ? '<div class="callout-grid"><a class="callout" href="' + href('authored/installation') + '"><small>GET STARTED</small><strong>Install px <span>↗</span></strong></a><a class="callout" href="' + href('authored/primitives') + '"><small>CORE CONCEPTS</small><strong>Explore the primitives <span>↗</span></strong></a></div>' : ''}<div class="article-body"></div>`;
  const body = article.querySelector('.article-body'); body.innerHTML = page.html;
  // The article title is already presented in the page header.
  if (!intro && body.firstElementChild && (body.firstElementChild.tagName === 'H1' || (body.firstElementChild.tagName === 'H2' && body.firstElementChild.textContent.trim().toLowerCase() === title.toLowerCase()))) body.firstElementChild.remove();
  body.querySelectorAll('a[href]').forEach(a => {
    const raw = a.getAttribute('href'); if (!raw || /^(?:[a-z]+:|#|\/\/)/i.test(raw)) return;
    const [path, fragment] = raw.split('#'); if (!path.endsWith('.md')) return;
    let resolved;
    if (path.startsWith('docs/')) resolved = path.slice(5, -3);
    else { const parts = key.split('/'); parts.pop(); for (const part of path.slice(0,-3).split('/')) { if (part === '..') parts.pop(); else if (part !== '.') parts.push(part); } resolved = parts.join('/'); }
    if (pages[resolved]) a.href = href(resolved) + (fragment ? '#' + fragment : '');
    else a.removeAttribute('href');
  });
  const toc = document.querySelector('#toc'); body.querySelectorAll('h2,h3').forEach(h => { if (!h.id) return; const a = document.createElement('a'); a.href = '#' + h.id; a.textContent = h.textContent; toc.append(a); });
  const all = groups.flatMap(([,items]) => items); const index = all.findIndex(([id]) => id === key); if (index >= 0 && all[index+1]) { const [id,label] = all[index+1]; const next = document.createElement('a'); next.className = 'next-link'; next.href = href(id); next.innerHTML = '<span><small>CONTINUE READING</small><strong></strong></span><span>↗</span>'; next.querySelector('strong').textContent = label; article.append(next); }
  const menu = document.querySelector('.menu-toggle'), sidebar = document.querySelector('#sidebar'); menu.addEventListener('click', () => { const open = sidebar.classList.toggle('open'); menu.setAttribute('aria-expanded', String(open)); });
  const dialog = document.querySelector('#search-dialog'), input = document.querySelector('#search-input'), results = document.querySelector('#search-results');
  function search() { const q = input.value.trim().toLowerCase(); results.replaceChildren(); if (!q) { results.textContent = 'Search titles and documentation content.'; return; } const matches = Object.entries(pages).filter(([id,p]) => (id + ' ' + p.title + ' ' + p.html.replace(/<[^>]*>/g,' ')).toLowerCase().includes(q)).slice(0,25); if (!matches.length) { results.textContent = 'No results found.'; return; } matches.forEach(([id,p]) => { const a = document.createElement('a'); a.href = href(id); a.textContent = labels[id] || p.title; const small = document.createElement('small'); small.textContent = p.source; a.append(small); results.append(a); }); }
  function openSearch() { dialog.showModal(); search(); input.focus(); }
  document.querySelector('.search-trigger').addEventListener('click',openSearch); document.querySelector('#search-close').addEventListener('click',() => dialog.close()); input.addEventListener('input',search); dialog.addEventListener('click', e => { if (e.target === dialog) dialog.close(); }); document.addEventListener('keydown', e => { if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); if (!dialog.open) openSearch(); } });
})();
