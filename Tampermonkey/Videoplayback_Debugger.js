// ==UserScript==
// @name         Google Drive Videoplayback Debugger (Test JSON)
// @namespace    http://tampermonkey.net/
// @version      4.3
// @description  Hỗ trợ bắt cả XHR và Fetch, hiển thị log nhận diện ngay khi khởi chạy.
// @author       GEMINI
// @match        *://*/*
// @grant        none
// @run-at       document-start
// ==/UserScript==

(function() {
    'use strict';

    function setupOverlay() {
        if (document.getElementById('gdv-debugger-overlay-v4')) return;
        if (!document.body) {
            setTimeout(setupOverlay, 100);
            return;
        }

        const overlay = document.createElement('div');
        overlay.id = 'gdv-debugger-overlay-v4';
        overlay.style.cssText = `
            position: fixed; top: 10px; right: 10px; width: 450px; height: 80vh;
            background: rgba(50,0,50,0.95); color: #0f0; font-family: monospace;
            font-size: 11px; z-index: 2147483647; padding: 10px; 
            border: 2px solid white; pointer-events: auto; word-wrap: break-word;
            display: flex; flex-direction: column;
        `;
        
        overlay.innerHTML = `
            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid #fff; padding-bottom:10px; margin-bottom:10px;">
                <h3 style="color:white; margin:0; font-size:14px;">🛠 TEST BẮT JSON V4.3 (Đệ quy)</h3>
                <div>
                    <button id="gdv-copy-btn" style="background:white; color:black; border:none; padding:5px 8px; cursor:pointer; font-weight:bold; border-radius:4px;">📋 Copy Log</button>
                </div>
            </div>
            <div id="gdv-logs" style="flex:1; overflow-y: auto;"></div>
        `;
        document.body.appendChild(overlay);

        document.getElementById('gdv-copy-btn').onclick = () => {
            const allLinks = Array.from(document.querySelectorAll('#gdv-logs span[data-fullurl]'))
                                     .map(span => span.dataset.fullurl)
                                     .join('\\n\\n');
            if (allLinks) {
                navigator.clipboard.writeText(allLinks).then(() => alert('Đã copy!')).catch(() => alert('Lỗi copy!'));
            }
        };

        logToBoard('INFO', 'Bảng Debugger đã sẵn sàng chờ lệnh...');
    }

    function logToBoard(type, msg) {
        // Gửi lên cửa sổ chính nếu đang ở trong iframe
        if (window.top !== window.self) {
            window.top.postMessage({ type: 'gdv_test_log', reqType: type, msg: msg }, '*');
            return;
        }

        const logContainer = document.getElementById('gdv-logs');
        if (!logContainer) return;

        const el = document.createElement('div');
        el.style.borderBottom = '1px dotted #444';
        el.style.marginBottom = '2px';
        el.style.paddingBottom = '2px';
        
        let color = '#fff';
        if (type === 'ERROR') color = '#ff3b30';
        if (type === 'SUCCESS') color = '#34c759';
        if (type.includes('VIDEO') || type.includes('AUDIO')) color = 'yellow';

        let displayMsg = msg.length > 200 ? msg.substring(0, 200) + '...' : msg;
        const safeMsg = displayMsg.replace(/</g, "&lt;").replace(/>/g, "&gt;");
        const fullSafeMsg = msg.replace(/</g, "&lt;").replace(/>/g, "&gt;");

        el.innerHTML = `<strong style="color: #ffaa00">[${type}]</strong> <span style="color:${color}" data-fullurl="${fullSafeMsg}">${safeMsg}</span>`;
        logContainer.appendChild(el);
        logContainer.scrollTop = logContainer.scrollHeight;
    }

    window.addEventListener('message', (e) => {
        if (e.data && e.data.type === 'gdv_test_log' && window.top === window.self) {
            logToBoard(e.data.reqType, e.data.msg);
        }
    });

    let alreadyTested = false;

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

    function testPlaybackJSON(text) {
        if (alreadyTested) return;
        alreadyTested = true;

        try {
            logToBoard('INFO', 'Đang phân tích dữ liệu JSON gốc...');
            
            if (!text || text.trim() === '') {
                logToBoard('ERROR', 'Dữ liệu trả về hoàn toàn trống (Empty String)!');
                return;
            }

            const data = JSON.parse(text);
            
            const formats = findStreamingData(data);

            if (!formats) {
                logToBoard('ERROR', 'Không tìm thấy "formatStreamingData" ở bất kỳ đâu trong JSON!');
                logToBoard('DEBUG', 'Cấu trúc gốc: ' + Object.keys(data).join(', '));
                return;
            }
            
            if (formats.progressiveTranscodes && formats.progressiveTranscodes.length > 0) {
                logToBoard('SUCCESS', 'TÌM THẤY ' + formats.progressiveTranscodes.length + ' LINK VIDEO ĐÃ GHÉP SẴN TIẾNG!');
                formats.progressiveTranscodes.forEach(s => {
                    const q = s.transcodeMetadata ? s.transcodeMetadata.height + 'p' : 'Unknown';
                    logToBoard('VIDEO_' + q, s.url);
                });
            } else {
                logToBoard('WARN', 'Không có progressiveTranscodes');
            }

            if (formats.adaptiveTranscodes && formats.adaptiveTranscodes.length > 0) {
                const audioStreams = formats.adaptiveTranscodes.filter(s => s.transcodeMetadata && s.transcodeMetadata.mimeType.includes('audio/'));
                if (audioStreams.length > 0) {
                    logToBoard('SUCCESS', 'TÌM THẤY LINK AUDIO!');
                    logToBoard('AUDIO', audioStreams[0].url);
                } else {
                    logToBoard('WARN', 'Có adaptiveTranscodes nhưng không có track audio riêng');
                }
            }
            
        } catch (e) {
            logToBoard('ERROR', 'Lỗi parse JSON: ' + e.message);
        }
    }

    const script = document.createElement('script');
    script.textContent = `
        (function() {
            function safePost(type, msg) {
                try { window.postMessage({ type: 'gdv_test_log', reqType: type, msg: msg }, '*'); } catch(e){}
            }

            safePost('INFO', 'Script ngầm đã được tiêm vào bộ nhớ. Đang chờ tải video...');

            // 1. Monkey patch Fetch
            const originalFetch = window.fetch;
            window.fetch = async function() {
                const url = arguments[0];
                let urlStr = typeof url === 'string' ? url : (url && url.url ? url.url : '');
                
                const responsePromise = originalFetch.apply(this, arguments);
                
                if(urlStr.includes('/v1/drive/media/') && urlStr.includes('/playback')) {
                    safePost('INFO', '[FETCH] Bắt được link Playback!');
                    
                    responsePromise.then(res => {
                        const clone = res.clone();
                        clone.text().then(text => {
                            window.postMessage({ type: 'gdv_trigger_test', text: text }, '*');
                        }).catch(e => safePost('ERROR', 'Lỗi đọc text Fetch: ' + e.message));
                    }).catch(e => safePost('ERROR', 'Fetch gốc bị lỗi: ' + e.message));
                }
                
                return responsePromise;
            };

            // 2. Monkey patch XHR (XMLHttpRequest)
            const originalXhrOpen = XMLHttpRequest.prototype.open;
            const originalXhrSend = XMLHttpRequest.prototype.send;

            XMLHttpRequest.prototype.open = function(method, url) {
                this._url = url;
                return originalXhrOpen.apply(this, arguments);
            };

            XMLHttpRequest.prototype.send = function() {
                const urlStr = typeof this._url === 'string' ? this._url : '';
                
                if (urlStr.includes('/v1/drive/media/') && urlStr.includes('/playback')) {
                    safePost('INFO', '[XHR] Bắt được link Playback!');
                    
                    this.addEventListener('load', function() {
                        safePost('INFO', '[XHR] Đã nhận phản hồi (Status ' + this.status + ')');
                        window.postMessage({ type: 'gdv_trigger_test', text: this.responseText }, '*');
                    });
                }
                
                return originalXhrSend.apply(this, arguments);
            };
        })();
    `;
    
    if (document.documentElement) {
        document.documentElement.appendChild(script);
        script.remove();
    }

    window.addEventListener('message', (e) => {
        if (e.data && e.data.type === 'gdv_trigger_test') {
            testPlaybackJSON(e.data.text);
        }
    });

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', setupOverlay);
    } else {
        setupOverlay();
    }
})();
