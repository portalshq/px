// Copy-to-clipboard for the install code blocks (PxCodeBlock port).
(function () {
  var ANALYTICS_CONSENT_KEY = 'portals_analytics_consent';
  var INSTALL_ID_KEY = 'portals_px_analytics_id';

  function analyticsId() {
    try {
      var existing = window.localStorage.getItem(INSTALL_ID_KEY);
      if (existing) return existing;
      var id = globalThis.crypto && globalThis.crypto.randomUUID
        ? globalThis.crypto.randomUUID()
        : 'px:' + Date.now().toString(36) + ':' + Math.random().toString(36).slice(2);
      window.localStorage.setItem(INSTALL_ID_KEY, id);
      return id;
    } catch (e) {
      return '';
    }
  }

  function anonymousDownloadId() {
    try {
      var key = 'portals_px_download_id';
      var existing = window.localStorage.getItem(key);
      if (existing && /^[0-9a-f]{32}$/.test(existing)) return existing;
      var id = globalThis.crypto && globalThis.crypto.randomUUID
        ? globalThis.crypto.randomUUID().replaceAll('-', '')
        : Array.from(globalThis.crypto.getRandomValues(new Uint8Array(16)), function (value) {
            return value.toString(16).padStart(2, '0');
          }).join('');
      window.localStorage.setItem(key, id);
      return id;
    } catch (e) {
      return '';
    }
  }

  function track(event, properties) {
    try {
      if (window.localStorage.getItem(ANALYTICS_CONSENT_KEY) !== 'accepted') return;
      var params = new URLSearchParams(window.location.search);
      var payload = {
        distinct_id: analyticsId(),
        source_page: window.location.pathname,
        utm_source: params.get('utm_source') || '',
        utm_medium: params.get('utm_medium') || '',
        utm_campaign: params.get('utm_campaign') || '',
        utm_content: params.get('utm_content') || '',
        ...properties
      };
      fetch('/api/analytics/track', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({event: event, properties: payload}),
        keepalive: true
      }).catch(function () {});
    } catch (e) {
      // Analytics must not affect the install or browsing experience.
    }
  }

  track('px_landing_viewed', {});

  var installCode = document.getElementById('px-code-1');
  var downloadId = anonymousDownloadId();
  var platform = 'unix';
  var pageParams = new URLSearchParams(window.location.search);
  var attribution = {
    utm_source: pageParams.get('utm_source') || 'website',
    utm_medium: pageParams.get('utm_medium') || 'website',
    utm_campaign: pageParams.get('utm_campaign') || 'px_launch',
    utm_content: pageParams.get('utm_content') || ''
  };

  function installCommand() {
    if (!installCode) return;
    var params = new URLSearchParams({platform: platform});
    if (downloadId) params.set('download_id', downloadId);
    Object.keys(attribution).forEach(function (key) {
      if (attribution[key]) params.set(key, attribution[key].slice(0, 160));
    });
    var url = 'https://portals.works/api/px/download?' + params.toString();
    installCode.textContent = platform === 'windows'
      ? "$env:PX_DOWNLOAD_ID='" + (downloadId || '') + "'; iex (irm '" + url + "')"
      : 'curl -fsSL "' + url + '" | PX_DOWNLOAD_ID=' + (downloadId || '') + ' bash';
  }

  document.querySelectorAll('.px-os-option').forEach(function (button) {
    button.addEventListener('click', function () {
      platform = button.getAttribute('data-platform') === 'windows' ? 'windows' : 'unix';
      document.querySelectorAll('.px-os-option').forEach(function (option) {
        option.setAttribute('aria-pressed', String(option === button));
      });
      installCommand();
    });
  });
  installCommand();

  var installSection = document.getElementById('install');
  if (installSection && 'IntersectionObserver' in window) {
    var installObserver = new IntersectionObserver(function (entries) {
      if (!entries.some(function (entry) { return entry.isIntersecting; })) return;
      track('px_install_section_viewed', {});
      installObserver.disconnect();
    }, {threshold: 0.25});
    installObserver.observe(installSection);
  }

  document.querySelectorAll('a[href*="github.com/portalshq/px"]').forEach(function (link) {
    link.addEventListener('click', function () {
      track('px_github_clicked', {placement: link.textContent.trim().slice(0, 80)});
    });
  });

  function setLabel(button, text) {
    button.textContent = text;
    button.setAttribute('aria-label', text === 'Copied' ? 'Copied command' : 'Copy command');
  }

  document.querySelectorAll('[data-copy]').forEach(function (button) {
    button.addEventListener('click', function () {
      var target = document.getElementById(button.getAttribute('data-copy'));
      if (!target) return;
      var text = target.textContent;
      function done(ok) {
        setLabel(button, ok ? 'Copied' : 'Copy');
        if (ok) {
          track('px_install_command_copied', {
            command_id: button.getAttribute('data-copy') || '',
            command_type: button.getAttribute('data-copy') === 'px-code-1' ? 'installer' : 'example'
          });
          window.setTimeout(function () { setLabel(button, 'Copy'); }, 1600);
        }
      }
      function legacyCopy() {
        var ta = document.createElement('textarea');
        ta.value = text;
        ta.setAttribute('readonly', '');
        ta.style.position = 'fixed';
        ta.style.opacity = '0';
        document.body.appendChild(ta);
        ta.select();
        var ok = false;
        try { ok = document.execCommand('copy'); } catch (e) { ok = false; }
        document.body.removeChild(ta);
        return ok;
      }
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(function () { done(true); }, function () { done(legacyCopy()); });
      } else {
        done(legacyCopy());
      }
    });
  });
})();

// Smooth scroll for same-page anchors (e.g. #how-it-works, #install).
(function () {
  document.addEventListener('click', function (e) {
    var a = e.target.closest ? e.target.closest('a[href^="#"]') : null;
    if (!a) return;
    var id = a.getAttribute('href');
    if (id.length < 2) return;
    var target = document.getElementById(id.slice(1));
    if (!target) return;
    e.preventDefault();
    target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    history.replaceState(null, '', id);
  });
})();
