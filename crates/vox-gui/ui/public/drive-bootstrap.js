(function () {
  try {
    if (new URLSearchParams(location.search).get('vox-drive') === '1') {
      window.__VOX_DRIVE_LIVE__ = true;
    }
  } catch (_err) {
    /* ignore */
  }
})();
