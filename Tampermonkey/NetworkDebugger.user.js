// ==UserScript==
// @name         Universal Network Debugger
// @namespace    http://tampermonkey.net/
// @version      1.1
// @description  Đánh chặn và log toàn bộ Fetch và XHR requests. Tích hợp nút Copy Log để dễ dàng trích xuất dữ liệu.
// @author       Bạn
// @match        *://*/*
// @grant        none
// @run-at       document-start
// ==/UserScript==

(function() {
    'use strict';

    // ==========================================
    // CẤU HÌNH & LƯU TRỮ LOG
    // ==========================================
    const reqStyle = 'color: #00bcd4; font-weight: bold;'; 
    const resStyle = 'color: #4caf50; font-weight: bold;'; 
    const errStyle = 'color: #f44336; font-weight: bold;'; 
    const infoStyle = 'color: #ff9800; font-size: 14px; font-weight: bold;';

    const allLogs = [];
    
    function saveLog(type, action, method, url, status, headers, bodyOrData) {
        // Giới hạn lưu trữ 500 log để tránh nặng máy
        if (allLogs.length > 500) allLogs.shift();
        
        allLogs.push({
            time: new Date().toLocaleTimeString(),
            type,       // 'FETCH' hoặc 'XHR'
            action,     // 'REQ' (Request), 'RES' (Response), 'ERR' (Error)
            method,
            url,
            status: status || 'N/A',
            headers: headers || null,
            data: bodyOrData || null
        });
    }

    // ==========================================
    // TẠO NÚT COPY LOG TRÊN GIAO DIỆN
    // ==========================================
    function createCopyButton() {
        if (document.getElementById('tm-copy-log-btn')) return;
        const btn = document.createElement('button');
        btn.id = 'tm-copy-log-btn';
        btn.innerText = '📋 Copy Logs';
        Object.assign(btn.style, {
            position: 'fixed',
            bottom: '20px',
            left: '20px', // Chuyển sang góc trái theo mặc định
            zIndex: '2147483647', // Max z-index
            padding: '10px 15px',
            background: '#2196F3',
            color: '#fff',
            border: 'none',
            borderRadius: '5px',
            cursor: 'grab',
            fontFamily: 'sans-serif',
            fontSize: '14px',
            boxShadow: '0 4px 6px rgba(0,0,0,0.3)',
            transition: 'background 0.3s',
            userSelect: 'none' // Chống bôi đen khi kéo
        });
        
        // --- LOGIC KÉO THẢ NÚT (DRAGGABLE) ---
        let isDragging = false;
        let isDragged = false; // Phân biệt click và kéo
        let startX, startY, initialLeft, initialBottom;

        btn.addEventListener('mousedown', (e) => {
            isDragging = true;
            isDragged = false;
            btn.style.cursor = 'grabbing';
            btn.style.transition = 'none'; // Tắt mượt mà để kéo không bị lag
            
            const rect = btn.getBoundingClientRect();
            startX = e.clientX;
            startY = e.clientY;
            initialLeft = rect.left;
            initialBottom = window.innerHeight - rect.bottom;
            
            e.preventDefault();
        });

        document.addEventListener('mousemove', (e) => {
            if (!isDragging) return;
            
            const dx = e.clientX - startX;
            const dy = e.clientY - startY;
            
            // Nếu di chuyển chuột > 3px thì xem như là đang kéo
            if (Math.abs(dx) > 3 || Math.abs(dy) > 3) isDragged = true;
            
            btn.style.left = `${initialLeft + dx}px`;
            btn.style.bottom = `${initialBottom - dy}px`;
            btn.style.right = 'auto'; // Xoá right để xài left
        });

        document.addEventListener('mouseup', () => {
            if (isDragging) {
                isDragging = false;
                btn.style.cursor = 'grab';
                btn.style.transition = 'background 0.3s';
            }
        });

        // --- GIAO DIỆN & CLICK ---
        btn.onmouseover = () => { if (!isDragging) btn.style.background = '#1976D2'; };
        btn.onmouseout = () => { if (!isDragging) btn.style.background = '#2196F3'; };
        
        btn.onclick = (e) => {
            if (isDragged) {
                e.preventDefault();
                return; // Nếu vừa kéo xong thì bỏ qua lệnh click
            }

            if (allLogs.length === 0) {
                alert('Chưa có log nào được ghi nhận!');
                return;
            }
            const textToCopy = JSON.stringify(allLogs, null, 2);
            navigator.clipboard.writeText(textToCopy).then(() => {
                const originalText = btn.innerText;
                btn.innerText = `✅ Đã copy (${allLogs.length} logs)!`;
                btn.style.background = '#4CAF50';
                setTimeout(() => { 
                    btn.innerText = originalText; 
                    btn.style.background = '#2196F3';
                }, 2000);
            }).catch(err => {
                console.error('Failed to copy logs', err);
                alert('Lỗi: Không thể copy log. Kiểm tra lại quyền clipboard hoặc copy thủ công trong Console.');
            });
        };
        
        const appendBtn = () => {
            if (!document.getElementById('tm-copy-log-btn') && document.body) {
                document.body.appendChild(btn);
            }
        };

        if (document.body) {
            appendBtn();
        } else {
            window.addEventListener('DOMContentLoaded', appendBtn);
        }
        
        // Cố gắng giữ nút copy không bị mất khi React/Vue re-render toàn bộ body
        const observer = new MutationObserver(() => {
            if (document.body && !document.getElementById('tm-copy-log-btn')) {
                appendBtn();
            }
        });
        observer.observe(document.documentElement, { childList: true, subtree: true });
    }

    createCopyButton();

    // ==========================================
    // 1. ĐÁNH CHẶN FETCH API
    // ==========================================
    const originalFetch = window.fetch;
    window.fetch = async function(...args) {
        const requestUrl = typeof args[0] === 'string' ? args[0] : (args[0] instanceof Request ? args[0].url : String(args[0]));
        const requestOptions = args[1] || {};
        const method = requestOptions.method || (args[0] instanceof Request ? args[0].method : 'GET');

        // Lưu & Log Request
        saveLog('FETCH', 'REQ', method, requestUrl, null, requestOptions.headers, requestOptions.body);
        
        console.groupCollapsed(`%c[FETCH REQ] ${method} ${requestUrl}`, reqStyle);
        console.log('Options/Headers:', requestOptions);
        if (args[0] instanceof Request) console.log('Request Object:', args[0]);
        console.groupEnd();

        try {
            const response = await originalFetch.apply(this, args);
            const clone = response.clone(); 
            
            // Lưu & Log Response
            clone.text().then(text => {
                const resHeaders = Object.fromEntries(response.headers.entries());
                let parsedData = text;
                try { parsedData = JSON.parse(text); } catch (e) {}
                
                saveLog('FETCH', 'RES', method, requestUrl, response.status, resHeaders, parsedData);

                console.groupCollapsed(`%c[FETCH RES] ${method} ${requestUrl} (${response.status})`, resStyle);
                console.log('Headers:', resHeaders);
                console.log('Data:', parsedData);
                console.groupEnd();
            }).catch(e => {
                saveLog('FETCH', 'ERR', method, requestUrl, null, null, e.message);
                console.log(`%c[FETCH RES] Failed to parse body for ${requestUrl}`, errStyle, e);
            });

            return response;
        } catch (error) {
            saveLog('FETCH', 'ERR', method, requestUrl, null, null, error.message);
            console.error(`%c[FETCH ERR] ${method} ${requestUrl}`, errStyle, error);
            throw error;
        }
    };

    // ==========================================
    // 2. ĐÁNH CHẶN XMLHTTPREQUEST (XHR)
    // ==========================================
    const originalXhrOpen = XMLHttpRequest.prototype.open;
    const originalXhrSend = XMLHttpRequest.prototype.send;
    const originalXhrSetRequestHeader = XMLHttpRequest.prototype.setRequestHeader;

    XMLHttpRequest.prototype.open = function(method, url, async, user, password) {
        this._requestData = {
            method: method,
            url: url,
            headers: {}
        };
        return originalXhrOpen.apply(this, arguments);
    };

    XMLHttpRequest.prototype.setRequestHeader = function(header, value) {
        if (this._requestData) {
            this._requestData.headers[header] = value;
        }
        return originalXhrSetRequestHeader.apply(this, arguments);
    };

    XMLHttpRequest.prototype.send = function(body) {
        if (this._requestData) {
            this._requestData.body = body;
            
            // Lưu & Log Request
            saveLog('XHR', 'REQ', this._requestData.method, this._requestData.url, null, this._requestData.headers, body);

            console.groupCollapsed(`%c[XHR REQ] ${this._requestData.method} ${this._requestData.url}`, reqStyle);
            if (Object.keys(this._requestData.headers).length > 0) console.log('Headers:', this._requestData.headers);
            if (body) console.log('Body:', body);
            console.groupEnd();
        }

        this.addEventListener('load', function() {
            if (this._requestData) {
                let parsedData = this.response;
                if (this.responseType === '' || this.responseType === 'text') {
                    try { parsedData = JSON.parse(this.responseText); } catch(e) { parsedData = this.responseText; }
                }
                
                // Lưu log Response
                saveLog('XHR', 'RES', this._requestData.method, this._requestData.url, this.status, this.getAllResponseHeaders(), parsedData);

                console.groupCollapsed(`%c[XHR RES] ${this._requestData.method} ${this._requestData.url} (${this.status})`, resStyle);
                console.log('Response Headers:\n', this.getAllResponseHeaders());
                console.log('Data:', parsedData);
                console.groupEnd();
            }
        });

        this.addEventListener('error', function() {
            if (this._requestData) {
                saveLog('XHR', 'ERR', this._requestData.method, this._requestData.url, null, null, 'Network Error');
                console.error(`%c[XHR ERR] ${this._requestData.method} ${this._requestData.url}`, errStyle);
            }
        });

        return originalXhrSend.apply(this, arguments);
    };

    console.log("%c🚀 Universal Network Debugger is running! Added Copy Log Button...", infoStyle);
})();
