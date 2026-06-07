// ==UserScript==
// @name         Google Drive Video Downloader V2 - Auto Sniffing
// @namespace    http://tampermonkey.net/
// @version      2.6
// @description  Automatically detects and grabs videoplayback links from Google Drive for direct downloading. Works across iframes.
// @author       GEMINI
// @match        https://drive.google.com/*
// @match        https://docs.google.com/*
// @license      MIT
// @grant        GM_download
// @grant        GM_setValue
// @grant        GM_getValue
// @grant        GM_addValueChangeListener
// @grant        GM_xmlhttpRequest
// @run-at       document-end
// ==/UserScript==

(function () {
    'use strict';

    // State
    let videoUrl = null;
    let videoAdaptiveUrl = null;
    let audioUrl = null;
    let videoQuality = '';
    let videoAdaptiveQuality = '';
    let audioQuality = '';
    let originalFilename = 'gdrive_download';

    // UI Elements
    let widget = null;
    let btnVideo = null;
    let btnVideoAdaptive = null;
    let btnAudio = null;
    let statusText = null;

    const PARAMS_TO_REMOVE = ['range', 'rn', 'rbuf', 'cpn', 'c', 'cver', 'srfvp', 'ump', 'alr'];

    // Create Apple-style CSS
    function createStyles() {
        const style = document.createElement('style');
        style.id = 'gdv2-style';
        style.textContent = `
            .gdv2-widget {
                position: fixed !important;
                bottom: 24px !important;
                right: 24px !important;
                z-index: 2147483647 !important;
                background: rgba(255, 255, 255, 0.95) !important;
                backdrop-filter: blur(20px) !important;
                -webkit-backdrop-filter: blur(20px) !important;
                border: 1px solid rgba(0, 0, 0, 0.1) !important;
                border-radius: 16px !important;
                box-shadow: 0 10px 30px rgba(0, 0, 0, 0.15), 0 4px 10px rgba(0, 0, 0, 0.1) !important;
                padding: 16px !important;
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif !important;
                display: flex !important;
                flex-direction: column !important;
                gap: 12px !important;
                transition: all 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) !important;
                min-width: 250px !important;
            }

            .gdv2-header {
                display: flex !important;
                align-items: center !important;
                gap: 8px !important;
                font-size: 14px !important;
                font-weight: 600 !important;
                color: #1d1d1f !important;
            }

            .gdv2-status-dot {
                width: 10px !important;
                height: 10px !important;
                border-radius: 50% !important;
                background: #ff3b30 !important;
                box-shadow: 0 0 8px rgba(255, 59, 48, 0.5) !important;
                transition: background 0.3s ease, box-shadow 0.3s ease !important;
            }

            .gdv2-status-dot.ready {
                background: #34c759 !important;
                box-shadow: 0 0 8px rgba(52, 199, 89, 0.5) !important;
            }

            .gdv2-buttons {
                display: flex !important;
                flex-direction: column !important;
                gap: 8px !important;
                display: none !important;
            }

            .gdv2-buttons.visible {
                display: flex !important;
            }

            .gdv2-btn {
                padding: 8px 12px !important;
                border: none !important;
                border-radius: 10px !important;
                cursor: pointer !important;
                font-size: 13px !important;
                font-weight: 600 !important;
                font-family: inherit !important;
                transition: all 0.2s ease !important;
                display: flex !important;
                align-items: center !important;
                justify-content: flex-start !important;
                gap: 8px !important;
            }

            .gdv2-btn-video {
                background: rgba(0, 122, 255, 0.1) !important;
                color: #007AFF !important;
            }

            .gdv2-btn-video:hover {
                background: rgba(0, 122, 255, 0.2) !important;
            }

            .gdv2-btn-video-adaptive {
                background: rgba(255, 149, 0, 0.1) !important;
                color: #ff9500 !important;
            }

            .gdv2-btn-video-adaptive:hover {
                background: rgba(255, 149, 0, 0.2) !important;
            }

            .gdv2-btn-audio {
                background: rgba(52, 199, 89, 0.1) !important;
                color: #34c759 !important;
            }

            .gdv2-btn-audio:hover {
                background: rgba(52, 199, 89, 0.2) !important;
            }

            .gdv2-btn:disabled {
                opacity: 0.5 !important;
                cursor: not-allowed !important;
            }

            @media (prefers-color-scheme: dark) {
                .gdv2-widget {
                    background: rgba(28, 28, 30, 0.95) !important;
                    border-color: rgba(255, 255, 255, 0.1) !important;
                }
                .gdv2-header {
                    color: #f2f2f7 !important;
                }
            }
        `;
        document.head.appendChild(style);
    }

    function createWidget() {
        widget = document.createElement('div');
        widget.id = 'gdv2-widget-container';
        widget.className = 'gdv2-widget';
        widget.innerHTML = `
            <div class="gdv2-header">
                <div class="gdv2-status-dot" id="gdv2-dot"></div>
                <span id="gdv2-status-text">Bấm Play Video để dò link...</span>
            </div>
            <div class="gdv2-buttons" id="gdv2-buttons">
                <a class="gdv2-btn gdv2-btn-video" id="gdv2-btn-video" title="Video có sẵn hình và tiếng" style="display:none; text-decoration:none;" target="_blank">
                    <span>🎥</span> <span id="lbl-video">Video (Có Tiếng)</span>
                </a>
                <a class="gdv2-btn gdv2-btn-video-adaptive" id="gdv2-btn-video-adaptive" title="Video chất lượng cao nhất nhưng không có tiếng" style="display:none; text-decoration:none;" target="_blank">
                    <span>🎞️</span> <span id="lbl-video-adaptive">Video (Không Tiếng)</span>
                </a>
                <a class="gdv2-btn gdv2-btn-audio" id="gdv2-btn-audio" title="Chỉ lấy âm thanh" style="display:none; text-decoration:none;" target="_blank">
                    <span>🎧</span> <span id="lbl-audio">Âm thanh</span>
                </a>
            </div>
        `;
        document.body.appendChild(widget);

        btnVideo = document.getElementById('gdv2-btn-video');
        btnVideoAdaptive = document.getElementById('gdv2-btn-video-adaptive');
        btnAudio = document.getElementById('gdv2-btn-audio');
        statusText = document.getElementById('gdv2-status-text');
    }

    function cleanUrlForDownload(urlStr) {
        try {
            const urlObj = urlStr.startsWith('http') ? new URL(urlStr) : new URL(urlStr, window.location.origin);
            PARAMS_TO_REMOVE.forEach(param => {
                urlObj.searchParams.delete(param);
            });
            return urlObj.toString();
        } catch (error) {
            console.error('[GDV2 Debug] Lỗi clean URL:', error);
            return urlStr;
        }
    }

    function updateUIFromStorage(data) {
        if (!data || !data.url) return;

        let updated = false;

        if (data.type === 'video') {
            videoUrl = data.url;
            videoQuality = data.quality;
            if (btnVideo) {
                btnVideo.href = data.url;
                document.getElementById('lbl-video').innerText = `Video (Có Tiếng) - ${videoQuality}`;
                btnVideo.style.display = 'flex';
            }
            updated = true;
        } else if (data.type === 'video-adaptive') {
            videoAdaptiveUrl = data.url;
            videoAdaptiveQuality = data.quality;
            if (btnVideoAdaptive) {
                btnVideoAdaptive.href = data.url;
                document.getElementById('lbl-video-adaptive').innerText = `Video (Không Tiếng) - ${videoAdaptiveQuality}`;
                btnVideoAdaptive.style.display = 'flex';
            }
            updated = true;
        } else if (data.type === 'audio') {
            audioUrl = data.url;
            audioQuality = data.quality;
            if (btnAudio) {
                btnAudio.href = data.url;
                document.getElementById('lbl-audio').innerText = `Âm thanh (Audio)`;
                btnAudio.style.display = 'flex';
            }
            updated = true;
        }

        if (updated && statusText) {
            document.getElementById('gdv2-dot').classList.add('ready');
            document.getElementById('gdv2-buttons').classList.add('visible');
            statusText.innerText = 'Đã bắt được link trực tiếp!';
        }
    }

    function findStreamingData(obj) {
        if (!obj || typeof obj !== 'object') return null;
        if (obj.progressiveTranscodes || obj.adaptiveTranscodes) return obj;
        if (obj.formatStreamingData) return obj.formatStreamingData;
        for (const key in obj) {
            const found = findStreamingData(obj[key]);
            if (found) return found;
        }
        return null;
    }

    function processPlaybackJSON(text) {
        try {
            const data = JSON.parse(text);
            const formats = findStreamingData(data);
            if (!formats) return;

            if (formats.progressiveTranscodes && formats.progressiveTranscodes.length > 0) {
                const bestVideo = formats.progressiveTranscodes.reduce((prev, curr) => {
                    return ((prev.transcodeMetadata && prev.transcodeMetadata.height || 0) > (curr.transcodeMetadata && curr.transcodeMetadata.height || 0)) ? prev : curr;
                });

                const q = (bestVideo.transcodeMetadata && bestVideo.transcodeMetadata.height) ? bestVideo.transcodeMetadata.height + 'p' : 'Bản Gốc';
                const streamUrl = cleanUrlForDownload(bestVideo.url);
                const vData = { type: 'video', url: streamUrl, quality: q, time: Date.now() };
                GM_setValue('gdv2_videoUrl', vData);
                if (window.top === window.self) updateUIFromStorage(vData);
            }

            if (formats.adaptiveTranscodes && formats.adaptiveTranscodes.length > 0) {
                // Audio
                const audioStreams = formats.adaptiveTranscodes.filter(s => s.transcodeMetadata && s.transcodeMetadata.mimeType.includes('audio/'));
                if (audioStreams.length > 0) {
                    const streamUrl = cleanUrlForDownload(audioStreams[0].url);
                    const aData = { type: 'audio', url: streamUrl, quality: 'Audio', time: Date.now() };
                    GM_setValue('gdv2_audioUrl', aData);
                    if (window.top === window.self) updateUIFromStorage(aData);
                }

                // Video Adaptive (Không tiếng)
                const videoAdaptiveStreams = formats.adaptiveTranscodes.filter(s => s.transcodeMetadata && s.transcodeMetadata.mimeType.includes('video/'));
                if (videoAdaptiveStreams.length > 0) {
                    const bestVideoAdaptive = videoAdaptiveStreams.reduce((prev, curr) => {
                        return ((prev.transcodeMetadata && prev.transcodeMetadata.height || 0) > (curr.transcodeMetadata && curr.transcodeMetadata.height || 0)) ? prev : curr;
                    });
                    const q = (bestVideoAdaptive.transcodeMetadata && bestVideoAdaptive.transcodeMetadata.height) ? bestVideoAdaptive.transcodeMetadata.height + 'p' : 'Bản Gốc';
                    const streamUrl = cleanUrlForDownload(bestVideoAdaptive.url);
                    const vaData = { type: 'video-adaptive', url: streamUrl, quality: q, time: Date.now() };
                    GM_setValue('gdv2_videoAdaptiveUrl', vaData);
                    if (window.top === window.self) updateUIFromStorage(vaData);
                }
            }
        } catch (err) {
            console.error('[GDV2 Error] Lỗi parse JSON:', err);
        }
    }

    function processUrl(urlStr) {
        try {
            const url = urlStr.startsWith('http') ? new URL(urlStr) : new URL(urlStr, window.location.origin);

            // Xử lý link videoplayback truyền thống
            if (!urlStr.includes('videoplayback')) return;

            const mime = url.searchParams.get('mime') || '';
            const itag = url.searchParams.get('itag');

            if (!itag) {
                return;
            }

            const cleanUrl = cleanUrlForDownload(urlStr);
            let vQuality = '';

            const isAudio = mime.includes('audio/') || ['139', '140', '141', '256', '258'].includes(itag);
            const isVideo = mime.includes('video/') || !isAudio;

            if (isVideo) {
                if (itag === '136') vQuality = '720p';
                else if (itag === '137') vQuality = '1080p';
                else if (itag === '298') vQuality = '720p60';
                else if (itag === '299') vQuality = '1080p60';
                else if (itag === '18') vQuality = '360p';
                else if (itag === '22') vQuality = '720p';
                else vQuality = itag || 'Auto';

                // Phân loại: 18 và 22 là progressive (có tiếng), các itag khác thường là adaptive (không tiếng)
                const hasAudio = ['18', '22', '43', '59'].includes(itag);
                const type = hasAudio ? 'video' : 'video-adaptive';

                const data = { type: type, url: cleanUrl, quality: vQuality, time: Date.now() };
                if (type === 'video') GM_setValue('gdv2_videoUrl', data);
                else GM_setValue('gdv2_videoAdaptiveUrl', data);

                if (window.top === window.self) updateUIFromStorage(data);

            } else if (isAudio) {
                const data = { type: 'audio', url: cleanUrl, quality: 'Audio', time: Date.now() };
                GM_setValue('gdv2_audioUrl', data);

                if (window.top === window.self) updateUIFromStorage(data);
            }

        } catch (e) {
            console.error('Lỗi phân tích URL:', e);
        }
    }

    function injectSniffer() {
        const script = document.createElement('script');
        script.textContent = `
            (function() {
                const originalFetch = window.fetch;
                window.fetch = async function() {
                    const url = arguments[0];
                    let urlStr = typeof url === 'string' ? url : (url && url.url ? url.url : '');
                    
                    const responsePromise = originalFetch.apply(this, arguments);
                    
                    if(urlStr.includes('/v1/drive/media/') && urlStr.includes('/playback')) {
                        responsePromise.then(res => {
                            const clone = res.clone();
                            clone.text().then(text => {
                                window.postMessage({ type: 'gdv2_playback_json', text: text }, '*');
                            }).catch(e => console.error(e));
                        }).catch(e => console.error(e));
                    } else if (urlStr.includes('videoplayback')) {
                        window.postMessage({ type: 'gdv2_sniffed', url: urlStr }, '*');
                    }
                    
                    return responsePromise;
                };

                const originalXhrOpen = XMLHttpRequest.prototype.open;
                const originalXhrSend = XMLHttpRequest.prototype.send;

                XMLHttpRequest.prototype.open = function(method, url) {
                    this._url = url;
                    return originalXhrOpen.apply(this, arguments);
                };

                XMLHttpRequest.prototype.send = function() {
                    const urlStr = typeof this._url === 'string' ? this._url : '';
                    
                    if (urlStr.includes('/v1/drive/media/') && urlStr.includes('/playback')) {
                        this.addEventListener('load', function() {
                            window.postMessage({ type: 'gdv2_playback_json', text: this.responseText }, '*');
                        });
                    } else if (urlStr.includes('videoplayback')) {
                        window.postMessage({ type: 'gdv2_sniffed', url: urlStr }, '*');
                    }
                    
                    return originalXhrSend.apply(this, arguments);
                };
            })();
        `;
        document.documentElement.appendChild(script);
        script.remove();

        window.addEventListener('message', (event) => {
            if (event.data) {
                if (event.data.type === 'gdv2_sniffed') {
                    processUrl(event.data.url);
                } else if (event.data.type === 'gdv2_playback_json') {
                    processPlaybackJSON(event.data.text);
                }
            }
        });
    }

    function startObserver() {
        if (!window.PerformanceObserver) return;

        const observer = new PerformanceObserver((list) => {
            for (const entry of list.getEntries()) {
                if (entry.name && (entry.name.includes('videoplayback') || entry.name.includes('/playback'))) {
                    processUrl(entry.name);
                }
            }
        });

        observer.observe({ entryTypes: ['resource'] });
    }

    function checkUrlAndInject() {
        const isFileView = true;

        if (!document.getElementById('gdv2-style')) {
            createStyles();
        }

        if (isFileView) {
            if (!document.getElementById('gdv2-widget-container')) {
                createWidget();
            } else if (widget) {
                widget.style.display = 'flex';
            }
        }
    }

    function initialize() {
        // Luôn bật sniffer trên mọi frame (cửa sổ chính + iframe ẩn)
        startObserver();
        injectSniffer();

        // Nhưng UI chỉ được tạo ở cửa sổ chính (top window)
        if (window.top === window.self) {
            setInterval(checkUrlAndInject, 1000);
            checkUrlAndInject();

            // Lắng nghe tín hiệu từ các iframe gửi lên qua GM_setValue
            GM_addValueChangeListener('gdv2_videoUrl', function (name, old_value, new_value, remote) {
                if (new_value) updateUIFromStorage(new_value);
            });

            GM_addValueChangeListener('gdv2_videoAdaptiveUrl', function (name, old_value, new_value, remote) {
                if (new_value) updateUIFromStorage(new_value);
            });

            GM_addValueChangeListener('gdv2_audioUrl', function (name, old_value, new_value, remote) {
                if (new_value) updateUIFromStorage(new_value);
            });

            console.log('🍎 V2.5 Auto-Sniffer UI ready in Top Window!');
        } else {
            console.log('🍎 V2.5 Auto-Sniffer Observer running in Iframe!');
        }
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initialize);
    } else {
        initialize();
    }

})();
