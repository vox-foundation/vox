(function () {
  try {
    if (!window.__TAURI_EVENT_PLUGIN_INTERNALS__) {
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: function () {} };
    }
    if (new URLSearchParams(location.search).get('vox-drive') === '1') {
      window.__VOX_DRIVE_LIVE__ = true;
    }
  } catch (_err) {
    /* ignore */
  }
})();
