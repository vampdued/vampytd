document.addEventListener('DOMContentLoaded', () => {
  const DEFAULTS = {
    downloadMode: 'interactive',
    preferredCodec: 'auto',
    enableCookies: false,
    enableCinematicCrop: true,
    enableBlockAmbient: true,
    enableChannelVideos: true
  };

  const overallStatus = document.getElementById('overall-status');
  const overallText   = document.getElementById('overall-text');
  const testButton    = document.getElementById('test-button');
  const checkedTime   = document.getElementById('checked-time');
  const modeSelector  = document.getElementById('mode-selector');
  const codecSelector = document.getElementById('codec-selector');
  const cookiesToggle = document.getElementById('cookies-toggle');

  const cookieDropZone     = document.getElementById('cookie-drop-zone');
  const cookieFileInput    = document.getElementById('cookie-file-input');
  const btnImportCookies   = document.getElementById('btn-import-cookies');
  const btnClearCookies    = document.getElementById('btn-clear-cookies');
  const clearCookiesLabel  = document.getElementById('clear-cookies-label');
  const cookieStatusBadge  = document.getElementById('cookie-status-badge');
  const cookieStatusDetail = document.getElementById('cookie-status-detail');
  const cookieFeedback     = document.getElementById('cookie-feedback');

  const activeVideoCard           = document.getElementById('active-video-card');
  const emptyMediaCard            = document.getElementById('empty-media-card');
  const activeVideoTitle          = document.getElementById('active-video-title');
  const activeVideoBadge          = document.getElementById('active-video-badge');
  const playlistContextPill       = document.getElementById('playlist-context-pill');
  const activeDownloadBtn         = document.getElementById('active-download-btn');
  const activeDownloadPlaylistBtn = document.getElementById('active-download-playlist-btn');
  const activeDownloadLabel       = document.getElementById('active-download-label');
  const activePlaylistLabel       = document.getElementById('active-playlist-label');
  const downloadLiveRegion        = document.getElementById('download-live-region');

  const toolsStatus    = document.getElementById('tools-status');
  const toolsSummary   = document.getElementById('tools-summary');
  const toolsDetails   = document.getElementById('tools-details');
  const bridgeHelpNote = document.getElementById('bridge-help-note');

  const copyFolderBtn = document.getElementById('btn-copy-folder');
  const copyLabel     = document.getElementById('copy-label');
  const folderEl      = document.getElementById('download-folder');

  // YT tools switches
  const toggleCinematicCrop = document.getElementById('toggle-cinematic-crop');
  const toggleBlockAmbient  = document.getElementById('toggle-block-ambient');
  const toggleChannelVideos = document.getElementById('toggle-channel-videos');

  let currentTabUrl = '';
  let currentIsPurePlaylist = false;
  let clearConfirmTimer = null;
  let feedbackTimer = null;

  const NATIVE_HOST = 'com.vampytd.bridge';

  function isMixListId(listId) {
    return typeof listId === 'string' && /^rd/i.test(listId.trim());
  }

  function stripPlaylistParams(rawUrl) {
    try {
      const u = new URL(rawUrl);
      const host = u.hostname.toLowerCase();
      if ((host.includes('youtube.com') || host.includes('youtu.be')) && u.pathname !== '/playlist') {
        u.searchParams.delete('list');
        u.searchParams.delete('index');
        u.searchParams.delete('start_radio');
        u.searchParams.delete('pp');
        return u.toString();
      }
    } catch (_) {}
    return rawUrl;
  }

  function sendNative(payload) {
    return new Promise((resolve) => {
      try {
        chrome.runtime.sendNativeMessage(NATIVE_HOST, payload, (response) => {
          if (chrome.runtime.lastError) {
            resolve({ ok: false, error: chrome.runtime.lastError.message });
            return;
          }
          resolve(response || { ok: false, error: 'No response from desktop app' });
        });
      } catch (err) {
        resolve({ ok: false, error: err.message });
      }
    });
  }

  document.getElementById('version-text').textContent =
    `v${chrome.runtime.getManifest().version}`;

  function setValue(id, text, state = '') {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = text;
    el.className = `tool-val${state ? ` ${state}` : ''}`;
  }

  function renderTool(id, available) {
    if (available === undefined || available === null) {
      setValue(id, 'Unknown');
      return null;
    } else if (available) {
      setValue(id, typeof available === 'string' ? available : 'Ready', 'good');
      return true;
    } else {
      setValue(id, 'Missing', 'bad');
      return false;
    }
  }

  function renderDiagnostics(result) {
    const nativeOK = Boolean(result?.native?.ok);
    const details  = result?.details;

    if (nativeOK) {
      setValue('native-status', 'Connected', 'good');
      if (bridgeHelpNote) bridgeHelpNote.hidden = true;
    } else {
      setValue('native-status', 'Offline', 'bad');
      if (bridgeHelpNote) bridgeHelpNote.hidden = false;
    }

    const tYtdlp  = renderTool('ytdlp-status',  details?.ytDlp);
    const tFfmpeg = renderTool('ffmpeg-status', details?.ffmpeg);
    const jsRuntimeName = details?.jsRuntime || null;
    const tJS     = renderTool('node-status', jsRuntimeName ? jsRuntimeName : (details ? false : null));

    const allRequiredReady = nativeOK && tYtdlp === true && tFfmpeg === true && tJS === true;

    if (!nativeOK) {
      overallStatus.className = 'overall-status bad';
      overallText.textContent = 'Disconnected';
      if (toolsStatus && toolsSummary) {
        toolsStatus.textContent = 'Offline';
        toolsStatus.className = 'status-pill bad';
        toolsSummary.textContent = 'Desktop app is not reachable';
      }
      if (toolsDetails) toolsDetails.open = true;
    } else if (allRequiredReady) {
      overallStatus.className = 'overall-status good';
      overallText.textContent = 'Ready';
      if (toolsStatus && toolsSummary) {
        toolsStatus.textContent = 'All ready';
        toolsStatus.className = 'status-pill good';
        toolsSummary.textContent = `yt-dlp, FFmpeg, ${jsRuntimeName} ready`;
      }
      if (toolsDetails) toolsDetails.open = false;
    } else {
      overallStatus.className = 'overall-status warn';
      overallText.textContent = 'Setup needed';
      const missing = [];
      if (tYtdlp === false) missing.push('yt-dlp');
      if (tFfmpeg === false) missing.push('FFmpeg');
      if (tJS === false) missing.push('JS runtime');
      if (toolsStatus && toolsSummary) {
        toolsStatus.textContent = 'Attention';
        toolsStatus.className = 'status-pill warn';
        toolsSummary.textContent = missing.length
          ? `Missing: ${missing.join(', ')}`
          : 'One or more required tools need setup';
      }
      if (toolsDetails) toolsDetails.open = true;
    }

    const hasValidFolder = Boolean(details?.downloadDir);
    if (folderEl) {
      folderEl.textContent = hasValidFolder ? details.downloadDir : 'Unavailable while disconnected';
    }
    if (copyFolderBtn) {
      copyFolderBtn.disabled = !hasValidFolder;
    }

    checkedTime.textContent =
      `Checked ${new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}`;
  }

  function runDiagnostics() {
    testButton.disabled = true;
    testButton.textContent = 'Testing…';
    overallStatus.className = 'overall-status';
    overallText.textContent = 'Checking';

    sendNative({ action: 'diagnostics' }).then((response) => {
      if (!response || !response.ok) {
        renderDiagnostics(null);
        checkedTime.textContent = 'Could not reach VampYTD desktop app';
      } else {
        renderDiagnostics({ ok: true, native: { ok: true }, details: response });
      }
      testButton.disabled = false;
      testButton.textContent = 'Test connection';
      refreshCookieStatus();
    });
  }

  // --- Copy Folder Path to Clipboard ---
  function copyFolderPath() {
    const rawPath = folderEl?.textContent;
    if (!rawPath || copyFolderBtn?.disabled) return;

    navigator.clipboard.writeText(rawPath).then(() => {
      if (copyLabel) copyLabel.textContent = 'Copied';
      copyFolderBtn?.classList.add('copied');
      if (downloadLiveRegion) downloadLiveRegion.textContent = 'Destination folder path copied';
      setTimeout(() => {
        if (copyLabel) copyLabel.textContent = 'Copy';
        copyFolderBtn?.classList.remove('copied');
      }, 2000);
    }).catch(() => {});
  }

  // --- Inline Cookie Feedback (replaces blocking alert/confirm) ---
  function showCookieFeedback(message, isError = false) {
    if (!cookieFeedback) return;
    if (feedbackTimer) clearTimeout(feedbackTimer);
    cookieFeedback.textContent = message;
    cookieFeedback.className = `cookie-feedback ${isError ? 'bad' : 'good'}`;
    cookieFeedback.hidden = false;
    feedbackTimer = setTimeout(() => {
      cookieFeedback.hidden = true;
    }, 5000);
  }

  function resetClearButton() {
    if (clearConfirmTimer) {
      clearTimeout(clearConfirmTimer);
      clearConfirmTimer = null;
    }
    if (btnClearCookies) {
      delete btnClearCookies.dataset.confirming;
      btnClearCookies.classList.remove('confirming');
    }
    if (clearCookiesLabel) clearCookiesLabel.textContent = 'Clear';
  }

  // --- Cookie Management Handlers (YouTube-focused) ---
  async function refreshCookieStatus() {
    if (!cookieStatusBadge) return;
    resetClearButton();
    const response = await sendNative({ action: 'cookie_status' });
    if (!response || !response.ok) {
      cookieStatusBadge.textContent = 'Offline';
      cookieStatusBadge.className = 'status-pill bad';
      if (cookieStatusDetail) {
        cookieStatusDetail.textContent = 'Desktop app is disconnected';
      }
      if (btnClearCookies) btnClearCookies.hidden = true;
      return;
    }

    const yt = response.youtube;
    if (yt && yt.exists) {
      if (yt.valid) {
        cookieStatusBadge.textContent = 'Signed in';
        cookieStatusBadge.className = 'status-pill good';
        const daysText = (yt.daysRemaining !== null && yt.daysRemaining !== undefined)
          ? `${yt.daysRemaining} ${yt.daysRemaining === 1 ? 'day' : 'days'} left`
          : 'Active session';
        if (cookieStatusDetail) {
          cookieStatusDetail.textContent = `Active session • ${daysText}`;
        }
      } else {
        cookieStatusBadge.textContent = 'Expired';
        cookieStatusBadge.className = 'status-pill bad';
        if (cookieStatusDetail) cookieStatusDetail.textContent = 'Session expired • Re-import cookies.txt';
      }
      if (btnClearCookies) btnClearCookies.hidden = false;
    } else {
      cookieStatusBadge.textContent = 'Not set';
      cookieStatusBadge.className = 'status-pill';
      if (cookieStatusDetail) cookieStatusDetail.textContent = 'No YouTube cookies saved';
      if (btnClearCookies) btnClearCookies.hidden = true;
    }
  }

  function handleCookieFile(file) {
    if (!file) return;
    if (!file.name.toLowerCase().endsWith('.txt')) {
      showCookieFeedback('Please choose a Netscape cookies .txt file.', true);
      return;
    }

    if (cookieStatusDetail) cookieStatusDetail.textContent = 'Cleaning & saving…';
    const reader = new FileReader();
    reader.onload = async (e) => {
      const content = e.target.result;
      const res = await sendNative({ action: 'import_cookies', content, target: 'youtube' });
      if (!res || !res.ok) {
        const err = res?.error || 'Could not import cookies.';
        showCookieFeedback(err, true);
        refreshCookieStatus();
        return;
      }

      if (cookiesToggle && !cookiesToggle.checked) {
        cookiesToggle.checked = true;
        syncSwitchAria(cookiesToggle);
        chrome.storage.local.set({ enableCookies: true });
      }

      showCookieFeedback('YouTube cookies imported and cleaned.', false);
      refreshCookieStatus();
    };
    reader.readAsText(file);
  }

  if (copyFolderBtn) copyFolderBtn.addEventListener('click', copyFolderPath);

  // --- Active Tab Video & Theme Detection ---
  function inspectActiveTab() {
    if (!chrome.tabs?.query) return;

    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const activeTab = tabs?.[0];
      if (!activeTab || !activeTab.url) return;

      currentTabUrl = activeTab.url;
      let isSupported = false;
      let isPurePlaylist = false;
      let hasPlaylist = false;
      let siteName = 'Video';

      try {
        const u = new URL(activeTab.url);
        const host = u.hostname.toLowerCase();

        if (host.includes('youtube.com') || host.includes('youtu.be')) {
          const listId = u.searchParams.get('list');
          if (u.pathname === '/playlist' && listId) {
            isSupported = true;
            isPurePlaylist = true;
            siteName = 'Playlist';
          } else if (u.pathname === '/watch' || u.pathname.startsWith('/shorts/') || host === 'youtu.be') {
            isSupported = true;
            siteName = u.pathname.startsWith('/shorts/') ? 'Shorts' : 'YouTube';
            if ((u.pathname === '/watch' || host === 'youtu.be') && listId && !isMixListId(listId)) {
              hasPlaylist = true;
            }
          }

          // Match YouTube's active Light/Dark theme cleanly in both directions
          chrome.tabs.sendMessage(activeTab.id, { action: 'getTheme' }, (res) => {
            if (!chrome.runtime.lastError && typeof res?.isDark === 'boolean') {
              document.documentElement.dataset.theme = res.isDark ? 'dark' : 'light';
            }
          });
        } else if (host.includes('hotstar.com') || host.includes('jiohotstar.com')) {
          isSupported = true;
          siteName = 'Hotstar';
        }
      } catch (_) {}

      currentIsPurePlaylist = isPurePlaylist;

      if (isSupported && activeVideoCard) {
        activeVideoCard.hidden = false;
        if (emptyMediaCard) emptyMediaCard.hidden = true;
        let cleanTitle = activeTab.title || (isPurePlaylist ? 'Active playlist' : 'Active video');
        cleanTitle = cleanTitle.replace(/\s*-\s*YouTube$/, '').replace(/\s*\|\s*Hotstar$/, '');
        activeVideoTitle.textContent = cleanTitle;
        activeVideoBadge.textContent = siteName;
        activeVideoBadge.classList.toggle('hotstar', siteName === 'Hotstar');

        if (isPurePlaylist) {
          if (activeDownloadLabel) activeDownloadLabel.textContent = 'Download full playlist';
          if (activeDownloadPlaylistBtn) activeDownloadPlaylistBtn.hidden = true;
          if (playlistContextPill) playlistContextPill.hidden = true;
        } else if (hasPlaylist) {
          if (activeDownloadLabel) activeDownloadLabel.textContent = 'Download this video';
          if (activeDownloadPlaylistBtn) activeDownloadPlaylistBtn.hidden = false;
          if (playlistContextPill) playlistContextPill.hidden = false;
        } else {
          if (activeDownloadLabel) activeDownloadLabel.textContent = 'Download this video';
          if (activeDownloadPlaylistBtn) activeDownloadPlaylistBtn.hidden = true;
          if (playlistContextPill) playlistContextPill.hidden = true;
        }
      } else {
        if (activeVideoCard) activeVideoCard.hidden = true;
        if (emptyMediaCard) emptyMediaCard.hidden = false;
      }
    });
  }

  if (activeDownloadBtn) {
    activeDownloadBtn.addEventListener('click', () => {
      if (!currentTabUrl) return;

      const isPlaylistDownload = currentIsPurePlaylist;
      const defaultLabel = isPlaylistDownload ? 'Download full playlist' : 'Download this video';
      const targetUrl = isPlaylistDownload ? currentTabUrl : stripPlaylistParams(currentTabUrl);

      activeDownloadBtn.disabled = true;
      if (activeDownloadLabel) activeDownloadLabel.textContent = 'Sending…';

      chrome.runtime.sendMessage({ action: 'download', url: targetUrl, playlist: isPlaylistDownload }, (response) => {
        const err = chrome.runtime.lastError;
        const ok = !err && Boolean(response?.ok);

        activeDownloadBtn.classList.remove('state-ok', 'state-err');
        if (ok) {
          const msg = isPlaylistDownload ? 'Playlist sent' : 'Sent to VampYTD';
          if (activeDownloadLabel) activeDownloadLabel.textContent = `✓ ${msg}`;
          activeDownloadBtn.classList.add('state-ok');
          if (downloadLiveRegion) downloadLiveRegion.textContent = msg;
        } else {
          if (activeDownloadLabel) activeDownloadLabel.textContent = 'Desktop app offline';
          activeDownloadBtn.classList.add('state-err');
          if (downloadLiveRegion) downloadLiveRegion.textContent = 'VampYTD desktop app is offline';
        }

        setTimeout(() => {
          activeDownloadBtn.disabled = false;
          activeDownloadBtn.classList.remove('state-ok', 'state-err');
          if (activeDownloadLabel) activeDownloadLabel.textContent = defaultLabel;
        }, 2200);
      });
    });
  }

  if (activeDownloadPlaylistBtn) {
    activeDownloadPlaylistBtn.addEventListener('click', () => {
      if (!currentTabUrl) return;

      activeDownloadPlaylistBtn.disabled = true;
      if (activePlaylistLabel) activePlaylistLabel.textContent = 'Sending…';

      chrome.runtime.sendMessage({ action: 'download', url: currentTabUrl, playlist: true }, (response) => {
        const err = chrome.runtime.lastError;
        const ok = !err && Boolean(response?.ok);

        activeDownloadPlaylistBtn.classList.remove('state-ok', 'state-err');
        if (ok) {
          if (activePlaylistLabel) activePlaylistLabel.textContent = '✓ Playlist sent';
          activeDownloadPlaylistBtn.classList.add('state-ok');
          if (downloadLiveRegion) downloadLiveRegion.textContent = 'Playlist sent to VampYTD';
        } else {
          if (activePlaylistLabel) activePlaylistLabel.textContent = 'Desktop app offline';
          activeDownloadPlaylistBtn.classList.add('state-err');
          if (downloadLiveRegion) downloadLiveRegion.textContent = 'VampYTD desktop app is offline';
        }

        setTimeout(() => {
          activeDownloadPlaylistBtn.disabled = false;
          activeDownloadPlaylistBtn.classList.remove('state-ok', 'state-err');
          if (activePlaylistLabel) activePlaylistLabel.textContent = 'Download full playlist';
        }, 2200);
      });
    });
  }

  function syncSwitchAria(el) {
    if (el) el.setAttribute('aria-checked', el.checked ? 'true' : 'false');
  }

  function broadcastSettingsUpdate(patch) {
    chrome.tabs?.query({ active: true, currentWindow: true }, (tabs) => {
      const activeTab = tabs?.[0];
      if (activeTab?.id) {
        chrome.tabs.sendMessage(activeTab.id, {
          action: 'settingsUpdated',
          settings: patch
        }).catch(() => {});
      }
    });
  }

  // --- Tab Switching with Keyboard Navigation ---
  const tabs = Array.from(document.querySelectorAll('.tab'));

  function activateTab(tab) {
    tabs.forEach((t) => {
      const isActive = t === tab;
      t.classList.toggle('active', isActive);
      t.setAttribute('aria-selected', isActive ? 'true' : 'false');
      t.tabIndex = isActive ? 0 : -1;
    });

    document.querySelectorAll('.panel').forEach((panel) => {
      panel.hidden = panel.id !== tab.dataset.panel;
    });
  }

  tabs.forEach((tab) => {
    tab.addEventListener('click', () => activateTab(tab));

    tab.addEventListener('keydown', (e) => {
      let targetTab = null;
      const idx = tabs.indexOf(tab);

      if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
        targetTab = tabs[(idx + 1) % tabs.length];
      } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
        targetTab = tabs[(idx - 1 + tabs.length) % tabs.length];
      }

      if (targetTab) {
        e.preventDefault();
        targetTab.focus();
        activateTab(targetTab);
      }
    });
  });

  // --- Radio Groups with ARIA + Keyboard Navigation ---
  function setupRadioGroup(container, itemSelector, settingKey, attrKey) {
    const options = Array.from(container.querySelectorAll(itemSelector));

    function selectOption(option) {
      options.forEach((o) => {
        const isMatch = o === option;
        o.classList.toggle('active', isMatch);
        o.setAttribute('aria-checked', isMatch ? 'true' : 'false');
        o.tabIndex = isMatch ? 0 : -1;
      });
      chrome.storage.local.set({ [settingKey]: option.dataset[attrKey] });
    }

    container.addEventListener('click', (event) => {
      const option = event.target.closest(itemSelector);
      if (option) selectOption(option);
    });

    container.addEventListener('keydown', (e) => {
      const focused = document.activeElement.closest(itemSelector);
      const current = focused || options.find((o) => o.classList.contains('active')) || options[0];
      const idx = options.indexOf(current);
      let nextOption = null;

      if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
        nextOption = options[(idx + 1) % options.length];
      } else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
        nextOption = options[(idx - 1 + options.length) % options.length];
      } else if (e.key === ' ' || e.key === 'Enter') {
        if (focused) nextOption = focused;
      }

      if (nextOption) {
        e.preventDefault();
        nextOption.focus();
        selectOption(nextOption);
      }
    });

    return selectOption;
  }

  const setModeOption  = setupRadioGroup(modeSelector, '.option-chip', 'downloadMode', 'mode');
  const setCodecOption = setupRadioGroup(codecSelector, '.codec-chip', 'preferredCodec', 'codec');

  // --- Restore Saved Settings ---
  chrome.storage.local.get(DEFAULTS, (items) => {
    cookiesToggle.checked = items.enableCookies;
    syncSwitchAria(cookiesToggle);

    const savedModeEl = modeSelector.querySelector(`[data-mode="${items.downloadMode}"]`);
    if (savedModeEl) setModeOption(savedModeEl);

    const savedCodecEl = codecSelector.querySelector(`[data-codec="${items.preferredCodec}"]`);
    if (savedCodecEl) setCodecOption(savedCodecEl);

    if (toggleCinematicCrop) {
      toggleCinematicCrop.checked = Boolean(items.enableCinematicCrop);
      syncSwitchAria(toggleCinematicCrop);
    }
    if (toggleBlockAmbient) {
      toggleBlockAmbient.checked = Boolean(items.enableBlockAmbient);
      syncSwitchAria(toggleBlockAmbient);
    }
    if (toggleChannelVideos) {
      toggleChannelVideos.checked = Boolean(items.enableChannelVideos);
      syncSwitchAria(toggleChannelVideos);
    }
  });

  // --- Settings Listeners ---
  cookiesToggle.addEventListener('change', () => {
    syncSwitchAria(cookiesToggle);
    chrome.storage.local.set({ enableCookies: cookiesToggle.checked });
  });

  if (toggleCinematicCrop) {
    toggleCinematicCrop.addEventListener('change', () => {
      syncSwitchAria(toggleCinematicCrop);
      const val = toggleCinematicCrop.checked;
      chrome.storage.local.set({ enableCinematicCrop: val });
      broadcastSettingsUpdate({ enableCinematicCrop: val });
    });
  }

  if (toggleBlockAmbient) {
    toggleBlockAmbient.addEventListener('change', () => {
      syncSwitchAria(toggleBlockAmbient);
      const val = toggleBlockAmbient.checked;
      chrome.storage.local.set({ enableBlockAmbient: val });
      broadcastSettingsUpdate({ enableBlockAmbient: val });
    });
  }

  if (toggleChannelVideos) {
    toggleChannelVideos.addEventListener('change', () => {
      syncSwitchAria(toggleChannelVideos);
      const val = toggleChannelVideos.checked;
      chrome.storage.local.set({ enableChannelVideos: val });
      broadcastSettingsUpdate({ enableChannelVideos: val });
    });
  }

  testButton.addEventListener('click', runDiagnostics);

  if (btnImportCookies && cookieFileInput) {
    btnImportCookies.addEventListener('click', () => {
      cookieFileInput.value = '';
      cookieFileInput.click();
    });

    cookieFileInput.addEventListener('change', (e) => {
      const file = e.target.files?.[0];
      if (file) handleCookieFile(file);
    });
  }

  if (btnClearCookies) {
    btnClearCookies.addEventListener('click', async () => {
      if (btnClearCookies.dataset.confirming !== 'true') {
        btnClearCookies.dataset.confirming = 'true';
        btnClearCookies.classList.add('confirming');
        if (clearCookiesLabel) clearCookiesLabel.textContent = 'Confirm clear?';
        clearConfirmTimer = setTimeout(resetClearButton, 4000);
        return;
      }

      resetClearButton();
      await sendNative({ action: 'clear_cookies', target: 'youtube' });
      showCookieFeedback('Saved YouTube cookies cleared.', false);
      refreshCookieStatus();
    });
  }

  if (cookieDropZone) {
    cookieDropZone.addEventListener('dragover', (e) => {
      e.preventDefault();
      cookieDropZone.classList.add('drag-over');
    });
    cookieDropZone.addEventListener('dragleave', () => {
      cookieDropZone.classList.remove('drag-over');
    });
    cookieDropZone.addEventListener('drop', (e) => {
      e.preventDefault();
      cookieDropZone.classList.remove('drag-over');
      const file = e.dataTransfer.files?.[0];
      if (file) handleCookieFile(file);
    });
  }

  // Initialize
  inspectActiveTab();
  runDiagnostics();
  refreshCookieStatus();
});
