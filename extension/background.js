const NATIVE_HOST = 'com.vampytd.bridge';
const NATIVE_TIMEOUT_MS = 10_000;

// Returns true for auto-generated YouTube Radio/Mix list IDs (e.g. RD..., RDMM...).
function isMixListId(listId) {
    return typeof listId === 'string' && /^rd/i.test(listId.trim());
}

// Strips playlist query parameters so a watch URL downloads only the single video.
function stripPlaylistParams(rawUrl) {
    try {
        const u = new URL(rawUrl);
        const host = u.hostname.toLowerCase();
        if (host.includes('youtube.com') || host.includes('youtu.be')) {
            if (u.pathname !== '/playlist') {
                u.searchParams.delete('list');
                u.searchParams.delete('index');
                u.searchParams.delete('start_radio');
                u.searchParams.delete('pp');
                return u.toString();
            }
        }
    } catch (_) {}
    return rawUrl;
}

// Wraps chrome.runtime.sendNativeMessage with a hard timeout so the service
// worker never silently hangs waiting for a response that will never arrive.
function sendNative(payload) {
    return new Promise((resolve, reject) => {
        let settled = false;

        const timer = setTimeout(() => {
            if (settled) return;
            settled = true;
            reject(new Error('Desktop app timed out'));
        }, NATIVE_TIMEOUT_MS);

        chrome.runtime.sendNativeMessage(NATIVE_HOST, payload, (response) => {
            if (settled) return;
            settled = true;
            clearTimeout(timer);

            if (chrome.runtime.lastError) {
                reject(new Error(chrome.runtime.lastError.message));
                return;
            }
            if (!response) {
                reject(new Error('No response from VampYTD desktop app'));
                return;
            }
            resolve(response);
        });
    });
}

// Temporary badge feedback on toolbar icon for context menu or quick actions.
function showBadgeFeedback(ok) {
    if (!chrome.action?.setBadgeText) return;
    const text = ok ? '✓' : 'ERR';
    const color = ok ? '#047857' : '#E00019';
    chrome.action.setBadgeText({ text });
    chrome.action.setBadgeBackgroundColor({ color });
    setTimeout(() => {
        chrome.action.setBadgeText({ text: '' });
    }, 3000);
}

// Reads stored settings then dispatches the download payload to the native host.
function sendToBridge(url, isPlaylist = false, items = '') {
    return new Promise((resolve) => {
        chrome.storage.local.get({
            downloadMode: 'interactive',
            preferredCodec: 'auto',
            enableCookies: false
        }, (stored) => {
            // Auto-treat YouTube Mix/Radio lists as single videos when on a /watch page
            let effectivePlaylist = Boolean(isPlaylist);
            try {
                const parsed = new URL(url);
                const listId = parsed.searchParams.get('list');
                if (parsed.pathname !== '/playlist' && listId && isMixListId(listId)) {
                    effectivePlaylist = false;
                }
            } catch (_) {}

            const finalUrl = effectivePlaylist ? url : stripPlaylistParams(url);

            const payload = {
                url: finalUrl,
                mode: stored.downloadMode,
                codec: stored.preferredCodec,
                cookies: stored.enableCookies,
                playlist: effectivePlaylist
            };
            if (effectivePlaylist && items) {
                payload.items = items;
            }

            sendNative(payload)
                .then(() => {
                    console.log('[VampYTD] Download dispatched via native messaging:', payload);
                    resolve({ ok: true, transport: 'native' });
                })
                .catch((err) => {
                    console.error('[VampYTD] Native messaging failed:', err.message);
                    resolve({ ok: false, error: err.message });
                });
        });
    });
}

// Pings the native host and returns a unified diagnostics object.
async function getDiagnostics() {
    try {
        const result = await sendNative({ action: 'diagnostics' });
        return {
            ok: true,
            native: { ok: true },
            details: result
        };
    } catch (err) {
        return {
            ok: false,
            native: { ok: false, error: err.message },
            details: null
        };
    }
}

// --- Background Context Menu Setup ---
chrome.runtime.onInstalled.addListener(() => {
    chrome.contextMenus.removeAll(() => {
        // Right-click on a video link
        chrome.contextMenus.create({
            id: 'vampytd-download-link',
            title: 'Download video with VampYTD',
            contexts: ['link'],
            targetUrlPatterns: [
                '*://*.youtube.com/watch*',
                '*://*.youtube.com/v/*',
                '*://*.youtube.com/shorts/*',
                '*://youtu.be/*',
                '*://*.hotstar.com/*',
                '*://*.jiohotstar.com/*'
            ]
        });

        // Right-click on a playlist link
        chrome.contextMenus.create({
            id: 'vampytd-download-playlist-link',
            title: 'Download full playlist with VampYTD',
            contexts: ['link'],
            targetUrlPatterns: [
                '*://*.youtube.com/playlist*',
                '*://*.youtube.com/watch*list=*'
            ]
        });

        // Right-click on the active page itself (video)
        chrome.contextMenus.create({
            id: 'vampytd-download-page',
            title: 'Download this video with VampYTD',
            contexts: ['page'],
            documentUrlPatterns: [
                '*://*.youtube.com/watch*',
                '*://*.youtube.com/shorts*',
                '*://*.hotstar.com/*',
                '*://*.jiohotstar.com/*'
            ]
        });

        // Right-click on the active page itself (playlist page)
        chrome.contextMenus.create({
            id: 'vampytd-download-playlist-page',
            title: 'Download full playlist with VampYTD',
            contexts: ['page'],
            documentUrlPatterns: [
                '*://*.youtube.com/playlist*',
                '*://*.youtube.com/watch*list=*'
            ]
        });
    });
});

// --- Context Menu Click Handler ---
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
    const isPlaylistMenu =
        info.menuItemId === 'vampytd-download-playlist-link' ||
        info.menuItemId === 'vampytd-download-playlist-page';
    const isLinkMenu =
        info.menuItemId === 'vampytd-download-link' ||
        info.menuItemId === 'vampytd-download-playlist-link';

    const targetUrl = isLinkMenu ? info.linkUrl : (info.pageUrl || tab?.url);
    if (!targetUrl) return;

    const res = await sendToBridge(targetUrl, isPlaylistMenu);
    showBadgeFeedback(res.ok);

    if (tab?.id) {
        chrome.tabs.sendMessage(tab.id, {
            action: 'toast',
            ok: res.ok,
            message: res.ok
                ? (isPlaylistMenu ? 'Playlist sent to VampYTD' : 'Video sent to VampYTD')
                : 'VampYTD desktop app is offline'
        }).catch(() => {});
    }
});

// --- Message Listener (from content script & popup) ---
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message.action === 'diagnostics') {
        getDiagnostics().then(sendResponse);
        return true;
    }
    if (message.action === 'ping') {
        getDiagnostics().then((result) =>
            sendResponse({ ok: result.ok, transport: result.native.ok ? 'native' : undefined })
        );
        return true;
    }
    if (message.action === 'download' && message.url) {
        sendToBridge(message.url, Boolean(message.playlist), message.items || '').then(sendResponse);
        return true;
    }
    if (message.action === 'cookie_status') {
        sendNative({ action: 'cookie_status' })
            .then(sendResponse)
            .catch((err) => sendResponse({ ok: false, error: err.message }));
        return true;
    }
    if (message.action === 'import_cookies') {
        sendNative({ action: 'import_cookies', content: message.content, target: message.target || 'youtube' })
            .then(sendResponse)
            .catch((err) => sendResponse({ ok: false, error: err.message }));
        return true;
    }
    if (message.action === 'clear_cookies') {
        sendNative({ action: 'clear_cookies', target: message.target || 'youtube' })
            .then(sendResponse)
            .catch((err) => sendResponse({ ok: false, error: err.message }));
        return true;
    }
});
