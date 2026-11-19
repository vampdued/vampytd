// ============================================================
// VampYTD Extension Content Script: Downloader + YT Tools
// ============================================================

// --- SVG Icons ---
const downloadIconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="currentColor" focusable="false" aria-hidden="true"><path d="M12 2a1 1 0 0 0-1 1v11.586l-4.293-4.293a1 1 0 1 0-1.414 1.414L12 18.414l6.707-6.707a1 1 0 1 0-1.414-1.414L13 14.586V3a1 1 0 0 0-1-1Zm7 18H5a1 1 0 0 0 0 2h14a1 1 0 0 0 0-2Z"/></svg>`;
const checkIconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 -960 960 960" fill="currentColor" focusable="false" aria-hidden="true"><path d="M382-240 154-468l57-57 171 171 367-367 57 57-424 424Z"/></svg>`;
const cinemaIconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="currentColor" focusable="false" aria-hidden="true"><path d="M19 4H5c-1.11 0-2 .9-2 2v12c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 14H5V6h14v12zM6 8h12v2H6zm0 6h12v2H6z"/></svg>`;
const chevronDownSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" focusable="false" aria-hidden="true"><path d="M7.41 8.59 12 13.17l4.59-4.58L18 10l-6 6-6-6z"/></svg>`;
const videoMenuIconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" focusable="false" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>`;
const playlistMenuIconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" focusable="false" aria-hidden="true"><path d="M4 6h16v2H4zm0 5h16v2H4zm0 5h10v2H4zm14 0v6l5-3z"/></svg>`;

// --- Feature Settings (Default: Enabled) ---
let settings = {
    enableCinematicCrop: true,
    enableBlockAmbient: true,
    enableChannelVideos: true
};

// --- URL & Theme Helpers ---
function isYouTubeDark() {
    return document.documentElement.hasAttribute('dark') ||
        document.documentElement.getAttribute('theme') === 'dark' ||
        Boolean(document.body?.classList?.contains('dark-theme'));
}

function isMixListId(listId) {
    return typeof listId === 'string' && /^rd/i.test(listId.trim());
}

function hasActionablePlaylist(rawUrl) {
    try {
        const u = new URL(rawUrl);
        if (u.pathname !== '/watch') return false;
        const listId = u.searchParams.get('list');
        return Boolean(listId && !isMixListId(listId));
    } catch {
        return false;
    }
}

function stripPlaylistParams(rawUrl) {
    try {
        const u = new URL(rawUrl);
        if (u.pathname !== '/playlist') {
            u.searchParams.delete('list');
            u.searchParams.delete('index');
            u.searchParams.delete('start_radio');
            u.searchParams.delete('pp');
            return u.toString();
        }
    } catch {}
    return rawUrl;
}

// ============================================================
// 1. AMBIENT MODE + OVERLAYS BLOCKER & INJECTED UI STYLES
// ============================================================

const cleanStyle = document.createElement('style');
cleanStyle.id = 'yt-clean-cinematic-style';
cleanStyle.textContent = `
    /* YouTube Ambient / Cinematic glow background */
    #cinematics,
    #cinematics.ytd-watch-flexy,
    .ytp-glow-effect,
    .ytp-glow-canvas-container {
        display: none !important;
    }

    /* Legacy annotation-style overlays */
    .annotation,
    .annotation-type-custom {
        display: none !important;
    }

    /* Crisp active state for player cinema button without drop-shadow */
    #vampytd-cinema-btn.active,
    #vampytd-cinema-btn.active svg {
        filter: none !important;
        text-shadow: none !important;
        box-shadow: none !important;
    }
`;

const uiStyle = document.createElement('style');
uiStyle.id = 'vampytd-ui-style';
uiStyle.textContent = `
    #vampytd-watch-btn {
        position: relative;
        display: inline-flex;
        align-items: center;
        margin-inline-end: 8px;
    }
    #vampytd-watch-btn .vampytd-split-wrap {
        display: inline-flex;
        align-items: stretch;
        border-radius: 18px;
        overflow: visible;
        position: relative;
    }
    #vampytd-watch-btn.has-playlist .vampytd-main-btn {
        border-top-right-radius: 0 !important;
        border-bottom-right-radius: 0 !important;
        padding-inline-end: 10px !important;
    }
    #vampytd-watch-menu-btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 30px;
        min-height: 36px;
        border: 0;
        border-inline-start: 1px solid rgba(128, 128, 128, 0.28);
        border-radius: 0 18px 18px 0;
        cursor: pointer;
        padding: 0;
        color: inherit;
        transition: background 0.15s ease;
    }
    #vampytd-watch-menu-btn:focus-visible,
    #vampytd-shorts-btn:focus-visible,
    .vampytd-menu-item:focus-visible {
        outline: 2px solid #FF0A21;
        outline-offset: 2px;
    }
    #vampytd-playlist-popover {
        position: absolute;
        top: calc(100% + 6px);
        left: 0;
        min-width: 196px;
        padding: 6px;
        border-radius: 12px;
        z-index: 99999;
        display: flex;
        flex-direction: column;
        gap: 2px;
        font: 500 13px/1.35 "YouTube Sans", Roboto, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    #vampytd-playlist-popover[hidden] {
        display: none !important;
    }
    .vampytd-menu-item {
        display: flex;
        align-items: center;
        gap: 10px;
        width: 100%;
        padding: 8px 10px;
        border: 0;
        border-radius: 8px;
        background: transparent;
        color: inherit;
        font: inherit;
        text-align: left;
        cursor: pointer;
        transition: background 0.12s ease;
    }
    .vampytd-menu-item-meta {
        display: flex;
        flex-direction: column;
        gap: 1px;
    }
    .vampytd-menu-item-title {
        font-weight: 600;
        font-size: 13px;
    }
    .vampytd-menu-item-sub {
        font-size: 11px;
        opacity: 0.72;
    }
    @media (prefers-reduced-motion: reduce) {
        #vampytd-floating-toast,
        #vampytd-shorts-btn,
        #vampytd-watch-menu-btn,
        .vampytd-menu-item {
            transition-duration: 0.01ms !important;
        }
    }
`;

function ensureUiStyle() {
    const parent = document.head || document.documentElement;
    if (parent && !uiStyle.isConnected) {
        parent.appendChild(uiStyle);
    }
}

function syncAmbientBlocker() {
    const parent = document.head || document.documentElement;
    if (!parent) return;

    if (settings.enableBlockAmbient) {
        if (!cleanStyle.isConnected) parent.appendChild(cleanStyle);
    } else {
        if (cleanStyle.isConnected) cleanStyle.remove();
    }
}

// Early execution at document_start
ensureUiStyle();
syncAmbientBlocker();

// ============================================================
// 2. CHANNEL VIDEOS TAB REWRITE & REDIRECT
// ============================================================

function channelVideosURL(rawURL) {
    if (!settings.enableChannelVideos) return null;
    let url;
    try {
        url = new URL(rawURL, window.location.href);
    } catch {
        return null;
    }

    if (url.protocol !== 'https:' && url.protocol !== 'http:') return null;
    if (url.hostname !== 'www.youtube.com' && url.hostname !== 'youtube.com') return null;

    const path = url.pathname;
    let base = null;

    let match = path.match(/^\/(@[^/]+)(?:\/featured)?\/?$/);
    if (match) base = `/${match[1]}`;

    if (!base) {
        match = path.match(/^\/(channel\/[^/]+)(?:\/featured)?\/?$/);
        if (match) base = `/${match[1]}`;
    }

    if (!base) {
        match = path.match(/^\/(c\/[^/]+)(?:\/featured)?\/?$/);
        if (match) base = `/${match[1]}`;
    }

    if (!base) {
        match = path.match(/^\/(user\/[^/]+)(?:\/featured)?\/?$/);
        if (match) base = `/${match[1]}`;
    }

    if (!base) return null;

    url.pathname = `${base}/videos`;
    return url.href;
}

function processChannelLink(anchor) {
    if (!(anchor instanceof HTMLAnchorElement)) return;
    const href = anchor.href;
    if (!href) return;

    const target = channelVideosURL(href);
    if (target && target !== href) {
        anchor.href = target;
        anchor.dataset.ytChannelVideosProcessed = '1';
    }
}

function processNode(node) {
    if (!(node instanceof Element)) return;
    if (node.matches?.('a[href]')) processChannelLink(node);
    const links = node.querySelectorAll?.('a[href]');
    if (links) {
        for (const anchor of links) processChannelLink(anchor);
    }
}

function processAllChannelLinks() {
    if (!settings.enableChannelVideos) return;
    const links = document.querySelectorAll('a[href]');
    for (const anchor of links) processChannelLink(anchor);
}

function redirectCurrentChannelPage() {
    if (!settings.enableChannelVideos) return false;
    const target = channelVideosURL(window.location.href);
    if (target && target !== window.location.href) {
        window.location.replace(target);
        return true;
    }
    return false;
}

redirectCurrentChannelPage();

const channelLinkObserver = new MutationObserver((mutations) => {
    if (!settings.enableChannelVideos) return;
    for (const mutation of mutations) {
        for (const node of mutation.addedNodes) {
            processNode(node);
        }
    }
});

function startChannelObserver() {
    if (document.documentElement) {
        channelLinkObserver.observe(document.documentElement, { childList: true, subtree: true });
    }
}

startChannelObserver();

document.addEventListener('click', (event) => {
    if (!settings.enableChannelVideos) return;
    const anchor = event.target?.closest?.('a[href]');
    if (anchor) processChannelLink(anchor);
}, true);

// ============================================================
// 3. 2.39:1 CINEMATIC CROP ENGINE
// ============================================================

const CINEMA_RATIO = 2.39;
const HOTKEY = 'c';

let cinematic = false;
let video = null;
let player = null;
let resizeObserver = null;
let lastScale = null;
let applyQueued = false;

function findVideo() {
    const videos = document.querySelectorAll('video');
    let fallback = null;

    for (const candidate of videos) {
        const rect = candidate.getBoundingClientRect();
        if (rect.width < 150 || rect.height < 100) continue;

        const style = getComputedStyle(candidate);
        if (style.display === 'none' || style.visibility === 'hidden') continue;
        if (!candidate.paused && !candidate.ended) return candidate;
        fallback ??= candidate;
    }
    return fallback;
}

function findPlayer(videoElement) {
    if (!videoElement) return null;
    return videoElement.closest('.html5-video-player') || document.querySelector('#movie_player');
}

function detectPlayer() {
    const newVideo = findVideo();
    if (!newVideo) return;

    const newPlayer = findPlayer(newVideo);
    if (!newPlayer) return;

    const changed = newVideo !== video || newPlayer !== player;
    video = newVideo;
    player = newPlayer;

    if (changed) {
        lastScale = null;
        setupResizeObserver();
    }

    if (cinematic) scheduleApplyCrop();
    if (settings.enableCinematicCrop) {
        injectPlayerCropButton();
    } else {
        removePlayerCropButton();
    }
}

function getPlayerAspect() {
    if (!player) return 16 / 9;
    const rect = player.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return 16 / 9;
    return rect.width / rect.height;
}

function calculateScale() {
    if (!cinematic || !settings.enableCinematicCrop) return 1;
    const aspect = getPlayerAspect();
    if (aspect >= CINEMA_RATIO) return 1;
    return CINEMA_RATIO / aspect;
}

function applyCrop() {
    applyQueued = false;
    const currentVideo = findVideo();
    if (!currentVideo) return;

    const currentPlayer = findPlayer(currentVideo);
    if (!currentPlayer) return;

    if (currentVideo !== video || currentPlayer !== player) {
        video = currentVideo;
        player = currentPlayer;
        lastScale = null;
        setupResizeObserver();
    }

    if (!cinematic || !settings.enableCinematicCrop) {
        if (lastScale !== 1) {
            video.style.transform = '';
            video.style.transformOrigin = '';
            video.style.willChange = '';
            lastScale = 1;
        }
        return;
    }

    const scale = calculateScale();
    if (lastScale === scale && video.style.transform) return;

    lastScale = scale;
    video.style.transformOrigin = 'center center';
    video.style.willChange = 'transform';
    video.style.transform = `scale(${scale})`;
}

function scheduleApplyCrop() {
    if (applyQueued) return;
    applyQueued = true;
    requestAnimationFrame(applyCrop);
}

function toggleCinematic() {
    if (!settings.enableCinematicCrop) return;
    cinematic = !cinematic;
    lastScale = null;
    scheduleApplyCrop();
    updatePlayerCropButtonState();
    showFloatingToast(true, cinematic ? 'Cinematic crop: on (2.39:1)' : 'Cinematic crop: off');
}

function setupResizeObserver() {
    if (resizeObserver) resizeObserver.disconnect();
    if (!player) return;

    resizeObserver = new ResizeObserver(() => {
        if (cinematic) {
            lastScale = null;
            scheduleApplyCrop();
        }
    });
    resizeObserver.observe(player);
}

function removePlayerCropButton() {
    const existing = document.getElementById('vampytd-cinema-btn');
    if (existing) existing.remove();
}

function injectPlayerCropButton() {
    if (!settings.enableCinematicCrop) return;
    const rightControls = document.querySelector('.ytp-right-controls');
    if (!rightControls || document.getElementById('vampytd-cinema-btn')) return;

    const btn = document.createElement('button');
    btn.id = 'vampytd-cinema-btn';
    btn.className = 'ytp-button';
    btn.type = 'button';
    btn.setAttribute('aria-label', 'Toggle 2.39:1 cinematic crop (Alt+C)');
    btn.setAttribute('aria-pressed', cinematic ? 'true' : 'false');
    btn.title = 'Cinematic crop 2.39:1 (Alt+C)';
    btn.style.verticalAlign = 'top';
    btn.innerHTML = `<div style="width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;">${cinemaIconSvg}</div>`;

    btn.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        toggleCinematic();
    });

    try {
        const fsButton = rightControls.querySelector('.ytp-fullscreen-button');
        if (fsButton && fsButton.parentElement) {
            fsButton.parentElement.insertBefore(btn, fsButton);
        } else {
            rightControls.appendChild(btn);
        }
    } catch (_) {
        try {
            rightControls.appendChild(btn);
        } catch (_) {}
    }

    updatePlayerCropButtonState();
}

function updatePlayerCropButtonState() {
    const btn = document.querySelector('#vampytd-cinema-btn');
    if (!btn) return;
    btn.style.color = cinematic ? '#FF0A21' : '#ffffff';
    btn.style.opacity = cinematic ? '1' : '0.85';
    btn.classList.toggle('active', cinematic);
    btn.setAttribute('aria-pressed', cinematic ? 'true' : 'false');
    const svg = btn.querySelector('svg');
    if (cinematic) {
        btn.style.setProperty('filter', 'none', 'important');
        btn.style.setProperty('text-shadow', 'none', 'important');
        btn.style.setProperty('box-shadow', 'none', 'important');
        if (svg) {
            svg.style.setProperty('filter', 'none', 'important');
            svg.style.setProperty('text-shadow', 'none', 'important');
            svg.style.setProperty('box-shadow', 'none', 'important');
        }
    } else {
        btn.style.removeProperty('filter');
        btn.style.removeProperty('text-shadow');
        btn.style.removeProperty('box-shadow');
        if (svg) {
            svg.style.removeProperty('filter');
            svg.style.removeProperty('text-shadow');
            svg.style.removeProperty('box-shadow');
        }
    }
}

// Hotkey: Alt+C
document.addEventListener('keydown', (event) => {
    if (!settings.enableCinematicCrop) return;
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target?.isContentEditable) {
        return;
    }

    if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (event.key.toLowerCase() !== HOTKEY) return;

    event.preventDefault();
    event.stopPropagation();
    toggleCinematic();
}, true);

document.addEventListener('fullscreenchange', () => {
    if (!cinematic) return;
    lastScale = null;
    scheduleApplyCrop();
    requestAnimationFrame(scheduleApplyCrop);
    setTimeout(scheduleApplyCrop, 100);
    setTimeout(scheduleApplyCrop, 300);
    setTimeout(detectPlayer, 500);
});

window.addEventListener('resize', () => {
    if (cinematic) {
        lastScale = null;
        scheduleApplyCrop();
    }
}, { passive: true });

// ============================================================
// 4. VAMPYTD DOWNLOADER (Watch Split Button & Shorts Injection)
// ============================================================

function closePlaylistPopover(returnFocus = false) {
    const popover = document.getElementById('vampytd-playlist-popover');
    const menuBtn = document.getElementById('vampytd-watch-menu-btn');
    if (popover && !popover.hidden) {
        popover.hidden = true;
    }
    if (menuBtn) {
        menuBtn.setAttribute('aria-expanded', 'false');
        if (returnFocus) menuBtn.focus();
    }
}

function stylePlaylistPopoverTheme(popover) {
    const dark = isYouTubeDark();
    popover.style.background = dark ? '#1C1C22' : '#FFFFFF';
    popover.style.color = dark ? '#FFFFFF' : '#0E0E12';
    popover.style.border = dark ? '1px solid #2A2A32' : '1px solid #E2E2E7';
    popover.style.boxShadow = dark
        ? '0 10px 28px rgba(0, 0, 0, 0.55)'
        : '0 10px 28px rgba(14, 14, 18, 0.14)';

    const hoverBg = dark ? '#2C2C36' : '#F4F4F6';
    popover.querySelectorAll('.vampytd-menu-item').forEach((item) => {
        item.onmouseenter = () => { item.style.background = hoverBg; };
        item.onmouseleave = () => { item.style.background = 'transparent'; };
    });
}

function triggerDownload(url, buttonElement, originalContent, isShorts, isPlaylist = false) {
    if (buttonElement.dataset.vampytdBusy === 'true') return;
    buttonElement.dataset.vampytdBusy = 'true';

    // Immediate pending feedback
    if (!isShorts) {
        buttonElement.innerHTML = createWatchButtonContent(downloadIconSvg, 'Sending…');
    }

    const targetUrl = isPlaylist ? url : stripPlaylistParams(url);

    let settled = false;
    const watchdogTimer = setTimeout(() => {
        if (settled) return;
        settled = true;
        showResult(false, 'Timed out', 'VampYTD desktop app timed out');
    }, 10_000);

    const showResult = (succeeded, btnLabel, toastText) => {
        if (isShorts) {
            buttonElement.innerHTML = succeeded ? checkIconSvg : downloadIconSvg;
            buttonElement.style.transform = 'scale(1.08)';
        } else {
            buttonElement.innerHTML = createWatchButtonContent(succeeded ? checkIconSvg : downloadIconSvg, btnLabel);
        }

        showFloatingToast(succeeded, toastText);

        setTimeout(() => {
            buttonElement.innerHTML = originalContent;
            if (isShorts) buttonElement.style.transform = '';
            delete buttonElement.dataset.vampytdBusy;
        }, 2000);
    };

    try {
        chrome.runtime.sendMessage({ action: 'download', url: targetUrl, playlist: isPlaylist }, (response) => {
            if (settled) return;
            settled = true;
            clearTimeout(watchdogTimer);

            const runtimeError = chrome.runtime.lastError;
            if (runtimeError) {
                const needsReload = runtimeError.message?.toLowerCase().includes('context invalidated');
                showResult(
                    false,
                    needsReload ? 'Reload page' : 'Extension error',
                    needsReload ? 'Reload the page to reconnect VampYTD' : 'Extension error'
                );
                return;
            }

            const succeeded = Boolean(response && response.ok);
            const okToast = isPlaylist ? 'Playlist sent to VampYTD' : 'Video sent to VampYTD';
            showResult(
                succeeded,
                succeeded ? 'Sent' : 'Offline',
                succeeded ? okToast : 'VampYTD desktop app is offline'
            );
        });
    } catch (error) {
        if (settled) return;
        settled = true;
        clearTimeout(watchdogTimer);

        const needsReload = error instanceof Error && error.message.toLowerCase().includes('context invalidated');
        showResult(
            false,
            needsReload ? 'Reload page' : 'Extension error',
            needsReload ? 'Reload the page to reconnect VampYTD' : 'Extension error'
        );
    }
}

function syncWatchButtonPlaylistMode(buttonModel) {
    const playlistMode = hasActionablePlaylist(window.location.href);
    buttonModel.classList.toggle('has-playlist', playlistMode);

    const menuBtn = buttonModel.querySelector('#vampytd-watch-menu-btn');
    const mainBtn = buttonModel.querySelector('.vampytd-main-btn');
    if (menuBtn) {
        menuBtn.hidden = !playlistMode;
    }
    if (mainBtn) {
        const label = playlistMode ? 'Download this video with VampYTD' : 'Download with VampYTD';
        mainBtn.title = label;
        mainBtn.setAttribute('aria-label', label);
    }
    if (!playlistMode) {
        closePlaylistPopover(false);
    }
}

function injectWatchPageButton() {
    ensureUiStyle();
    const menuContainer =
        document.querySelector('ytd-watch-metadata #top-level-buttons-computed') ||
        document.querySelector('ytd-menu-renderer #top-level-buttons-computed');

    if (!menuContainer) return;

    const existing = menuContainer.querySelector('#vampytd-watch-btn');
    if (existing) {
        syncWatchButtonPlaylistMode(existing);
        return;
    }

    document.querySelectorAll('#vampytd-watch-btn').forEach((orphan) => {
        if (!menuContainer.contains(orphan)) orphan.remove();
    });

    const buttonModel = document.createElement('yt-button-view-model');
    buttonModel.id = 'vampytd-watch-btn';
    buttonModel.className = 'ytd-menu-renderer';
    buttonModel.innerHTML = `
        <div class="vampytd-split-wrap">
          <button-view-model class="ytSpecButtonViewModelHost style-scope ytd-menu-renderer">
            <button type="button" class="vampytd-main-btn ytSpecButtonShapeNextHost ytSpecButtonShapeNextTonal ytSpecButtonShapeNextMono ytSpecButtonShapeNextSizeM ytSpecButtonShapeNextIconLeading ytSpecButtonShapeNextEnableBackdropFilterExperiment" title="Download this video with VampYTD" aria-label="Download this video with VampYTD" aria-disabled="false">
              ${createWatchButtonContent(downloadIconSvg, 'VampYTD')}
            </button>
          </button-view-model>
          <button type="button" id="vampytd-watch-menu-btn" class="ytSpecButtonShapeNextHost ytSpecButtonShapeNextTonal ytSpecButtonShapeNextMono" aria-haspopup="menu" aria-expanded="false" aria-controls="vampytd-playlist-popover" aria-label="Choose video or full playlist" title="Choose video or full playlist" hidden>
            ${chevronDownSvg}
          </button>
          <div id="vampytd-playlist-popover" role="menu" aria-label="Download scope" hidden>
            <button type="button" class="vampytd-menu-item" role="menuitem" data-scope="video">
              ${videoMenuIconSvg}
              <span class="vampytd-menu-item-meta">
                <span class="vampytd-menu-item-title">This video only</span>
                <span class="vampytd-menu-item-sub">Strip playlist info</span>
              </span>
            </button>
            <button type="button" class="vampytd-menu-item" role="menuitem" data-scope="playlist">
              ${playlistMenuIconSvg}
              <span class="vampytd-menu-item-meta">
                <span class="vampytd-menu-item-title">Full playlist</span>
                <span class="vampytd-menu-item-sub">Download all videos in list</span>
              </span>
            </button>
          </div>
        </div>`;

    const mainBtn = buttonModel.querySelector('.vampytd-main-btn');
    const menuBtn = buttonModel.querySelector('#vampytd-watch-menu-btn');
    const popover = buttonModel.querySelector('#vampytd-playlist-popover');
    const menuItems = Array.from(popover.querySelectorAll('.vampytd-menu-item'));
    const originalContent = createWatchButtonContent(downloadIconSvg, 'VampYTD');

    mainBtn.addEventListener('click', (event) => {
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();
        closePlaylistPopover(false);
        triggerDownload(window.location.href, mainBtn, originalContent, false, false);
    }, true);

    menuBtn.addEventListener('click', (event) => {
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();
        const willOpen = popover.hidden;
        if (willOpen) {
            stylePlaylistPopoverTheme(popover);
            popover.hidden = false;
            menuBtn.setAttribute('aria-expanded', 'true');
            menuItems[0]?.focus();
        } else {
            closePlaylistPopover(false);
        }
    }, true);

    popover.addEventListener('keydown', (event) => {
        const idx = menuItems.indexOf(document.activeElement);
        if (event.key === 'Escape') {
            event.preventDefault();
            closePlaylistPopover(true);
        } else if (event.key === 'ArrowDown') {
            event.preventDefault();
            menuItems[(idx + 1) % menuItems.length]?.focus();
        } else if (event.key === 'ArrowUp') {
            event.preventDefault();
            menuItems[(idx - 1 + menuItems.length) % menuItems.length]?.focus();
        }
    });

    menuItems.forEach((item) => {
        item.addEventListener('click', (event) => {
            event.preventDefault();
            event.stopPropagation();
            const isPlaylist = item.dataset.scope === 'playlist';
            closePlaylistPopover(false);
            triggerDownload(window.location.href, mainBtn, originalContent, false, isPlaylist);
        });
    });

    syncWatchButtonPlaylistMode(buttonModel);
    menuContainer.appendChild(buttonModel);
}

// Close playlist popover on outside click
document.addEventListener('click', (event) => {
    const popover = document.getElementById('vampytd-playlist-popover');
    if (!popover || popover.hidden) return;
    if (!event.target?.closest?.('#vampytd-watch-btn')) {
        closePlaylistPopover(false);
    }
});

function createWatchButtonContent(icon, label) {
    return `
        <div aria-hidden="true" class="ytSpecButtonShapeNextIcon ytSpecButtonShapeNextElevatedContent"><span class="ytIconWrapperHost" style="width: 24px; height: 24px;"><span class="yt-icon-shape ytSpecIconShapeHost"><div style="width: 100%; height: 100%; display: block; fill: currentcolor;">${icon}</div></span></span></div>
        <div class="ytSpecButtonShapeNextButtonTextContent ytSpecButtonShapeNextElevatedContent">${label}</div>
        <yt-touch-feedback-shape aria-hidden="true" class="ytSpecTouchFeedbackShapeHost ytSpecTouchFeedbackShapeTouchResponse"><div class="ytSpecTouchFeedbackShapeStroke"></div><div class="ytSpecTouchFeedbackShapeFill"></div></yt-touch-feedback-shape>
        <yt-light-shape aria-hidden="true" class="contribYtLightShapeHost contribYtLightShapeStaticRimLight contribYtLightShapeStaticRimLightTonal"><div class="contribYtLightShapeStaticWashLight contribYtLightShapeStaticWashLightTonal"></div></yt-light-shape>`;
}

function applyShortsButtonTheme(btn) {
    const dark = isYouTubeDark();
    btn.style.background = dark ? 'rgba(255, 255, 255, 0.12)' : 'rgba(14, 14, 18, 0.08)';
    btn.style.color = dark ? '#ffffff' : '#0E0E12';
}

function injectShortsButton() {
    ensureUiStyle();
    const actionPanel =
        document.querySelector('ytd-reel-video-renderer[is-active] #actions') ||
        document.querySelector('ytd-shorts #actions');

    if (!actionPanel) return;
    const existing = actionPanel.querySelector('#vampytd-shorts-btn');
    if (existing) {
        applyShortsButtonTheme(existing);
        return;
    }

    const btn = document.createElement('button');
    btn.id = 'vampytd-shorts-btn';
    btn.type = 'button';
    btn.setAttribute('aria-label', 'Download Short with VampYTD');
    btn.title = 'Download Short with VampYTD';
    btn.innerHTML = downloadIconSvg;

    Object.assign(btn.style, {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '48px',
        height: '48px',
        borderRadius: '50%',
        border: 'none',
        cursor: 'pointer',
        margin: '8px 0',
        backdropFilter: 'blur(8px)',
        transition: 'background 0.2s, transform 0.15s'
    });
    applyShortsButtonTheme(btn);

    btn.addEventListener('mouseenter', () => {
        btn.style.background = isYouTubeDark() ? 'rgba(255, 255, 255, 0.22)' : 'rgba(14, 14, 18, 0.16)';
    });
    btn.addEventListener('mouseleave', () => {
        applyShortsButtonTheme(btn);
    });

    const originalContent = downloadIconSvg;

    btn.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        triggerDownload(window.location.href, btn, originalContent, true, false);
    });

    actionPanel.appendChild(btn);
}

// In-Page Floating Toast Notification (Dynamic YouTube Theme Matching + ARIA)
function showFloatingToast(ok, text) {
    const existing = document.getElementById('vampytd-floating-toast');
    if (existing) existing.remove();

    const isDark = isYouTubeDark() || window.matchMedia('(prefers-color-scheme: dark)').matches;
    const isCinematic = text && text.toLowerCase().includes('cinematic');

    const bg = isDark ? 'rgba(14, 14, 18, 0.95)' : 'rgba(255, 255, 255, 0.96)';
    const color = isDark ? '#ffffff' : '#0E0E12';
    const border = isDark ? '1px solid #2A2A32' : '1px solid #E2E2E7';
    const shadow = isDark
        ? '0 8px 24px rgba(0, 0, 0, 0.6), 0 2px 6px rgba(0, 0, 0, 0.4)'
        : '0 8px 24px rgba(0, 0, 0, 0.12), 0 2px 6px rgba(0, 0, 0, 0.06)';

    let iconSvg = '';
    if (isCinematic) {
        iconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="#FF0A21" aria-hidden="true"><path d="M19 4H5c-1.11 0-2 .9-2 2v12c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 14H5V6h14v12zM6 8h12v2H6zm0 6h12v2H6z"/></svg>`;
    } else if (ok) {
        const iconColor = isDark ? '#34D399' : '#047857';
        iconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="${iconColor}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>`;
    } else {
        const iconColor = isDark ? '#F87171' : '#E00019';
        iconSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="${iconColor}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>`;
    }

    const toast = document.createElement('div');
    toast.id = 'vampytd-floating-toast';
    toast.setAttribute('role', ok ? 'status' : 'alert');
    toast.setAttribute('aria-live', ok ? 'polite' : 'assertive');
    toast.style.cssText = `
        position: fixed;
        bottom: 24px;
        right: 24px;
        z-index: 999999;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px 16px 8px 12px;
        border-radius: 9999px;
        background: ${bg};
        color: ${color};
        border: ${border};
        font: 500 13px/1.4 "YouTube Sans", Roboto, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        box-shadow: ${shadow};
        backdrop-filter: blur(12px);
        -webkit-backdrop-filter: blur(12px);
        opacity: 0;
        transform: translateY(12px) scale(0.96);
        transition: opacity 0.25s cubic-bezier(0.16, 1, 0.3, 1), transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
        pointer-events: none;
        user-select: none;
    `;

    const iconSpan = document.createElement('span');
    iconSpan.style.cssText = 'display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0;';
    iconSpan.innerHTML = iconSvg;

    const textSpan = document.createElement('span');
    textSpan.textContent = text;
    textSpan.style.cssText = 'white-space: nowrap;';

    toast.appendChild(iconSpan);
    toast.appendChild(textSpan);
    document.body.appendChild(toast);

    requestAnimationFrame(() => {
        toast.style.opacity = '1';
        toast.style.transform = 'translateY(0) scale(1)';
    });

    setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateY(8px) scale(0.96)';
        setTimeout(() => toast.remove(), 250);
    }, 3000);
}

// ============================================================
// 5. LIFECYCLE, OBSERVATION & SETTINGS SYNC
// ============================================================

function injectForCurrentPage() {
    const path = window.location.pathname;
    if (path === '/watch') {
        injectWatchPageButton();
        detectPlayer();
    } else if (path.startsWith('/shorts/')) {
        injectShortsButton();
    }
}

let debounceTimer = null;
const observer = new MutationObserver(() => {
    if (debounceTimer !== null) return;
    debounceTimer = setTimeout(() => {
        debounceTimer = null;
        injectForCurrentPage();
    }, 150);
});

if (document.body) {
    observer.observe(document.body, { childList: true, subtree: true });
} else {
    document.addEventListener('DOMContentLoaded', () => {
        observer.observe(document.body, { childList: true, subtree: true });
    }, { once: true });
}

document.addEventListener('yt-navigate-finish', () => {
    closePlaylistPopover(false);
    processAllChannelLinks();
    redirectCurrentChannelPage();

    if (debounceTimer !== null) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
    }
    injectForCurrentPage();
    setTimeout(detectPlayer, 0);
});

document.addEventListener('yt-page-data-fetched', () => {
    processAllChannelLinks();
});

document.addEventListener('DOMContentLoaded', () => {
    ensureUiStyle();
    startChannelObserver();
    processAllChannelLinks();
    detectPlayer();
    injectForCurrentPage();
}, { once: true });

setInterval(() => {
    processAllChannelLinks();
    detectPlayer();
}, 2000);

chrome.storage.local.get(settings, (items) => {
    if (items) {
        settings = { ...settings, ...items };
        syncAmbientBlocker();
        if (settings.enableCinematicCrop) {
            detectPlayer();
        } else {
            removePlayerCropButton();
        }
    }
});

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (msg.action === 'getTheme') {
        sendResponse({ isDark: isYouTubeDark() });
        return true;
    } else if (msg.action === 'toast') {
        showFloatingToast(msg.ok, msg.message);
    } else if (msg.action === 'settingsUpdated' && msg.settings) {
        settings = { ...settings, ...msg.settings };
        syncAmbientBlocker();
        if (!settings.enableCinematicCrop) {
            if (cinematic) {
                cinematic = false;
                applyCrop();
            }
            removePlayerCropButton();
        } else {
            detectPlayer();
        }
    }
});
