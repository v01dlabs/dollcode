(async function() {
    // Timing constants
    const TIMING = {
        CHAR_ANIMATION: 6,
        DELETE_SPEED: 8,
        DELETE_SPEED_FAST: 4,
        DEBOUNCE_DELAY: 60,
        RESIZE_THROTTLE: 10,
        MIN_HEIGHT: 80,
        HELD_THRESHOLD: 300,
        MAX_RETRIES: 5,
        RETRY_DELAY: 200,
        COPY_FEEDBACK_DURATION: 1200,
        SHAKE_DURATION: 400,
        LOAD_TIMEOUT: 10000
    };

    // Application states
    const State = {
        IDLE: 'idle',
        TYPING: 'typing',
        DELETING: 'deleting',
        ERROR: 'error'
    };

    // Key state management
    const KeyState = {
        _heldKeys: new Map(),

        isKeyHeld(key) {
            return this._heldKeys.has(key);
        },

        getHoldTime(key) {
            const startTime = this._heldKeys.get(key);
            return startTime ? Date.now() - startTime : 0;
        },

        isHoldThreshold(key) {
            return this.getHoldTime(key) > TIMING.HELD_THRESHOLD;
        },

        pressKey(key) {
            if (!this._heldKeys.has(key)) {
                this._heldKeys.set(key, Date.now());
            }
        },

        releaseKey(key) {
            this._heldKeys.delete(key);
        },

        clear() {
            this._heldKeys.clear();
        }
    };

    class OutputManager {
        constructor(outputElement, wasmFunctions) {
            this.output = outputElement;
            this.wasm = wasmFunctions;
            this.state = State.IDLE;
            this.contentDiv = null;
            this.copyButton = null;
            this.chars = [];
            this.lastProcessedInput = '';
            this.currentAnimation = null;
            this.debounceTimeout = null;
            this.isProcessing = false;
            this.resizeObserver = null;

            this.setupDOM();
            this.setupEventListeners();

            // Initial content
            this.contentDiv.textContent = '▌▖▌▖‍▌▘▌▌‍▌▘▘▌‍▌▘▘▌‍▌▖▘▌‍▌▘▌▌‍▌▖▌▖‍▌▖▌▘‍';
        }

        setupDOM() {
            // Create content div
            this.contentDiv = document.createElement('div');
            this.contentDiv.className = 'output-content';

            // Create copy button
            this.copyButton = document.createElement('button');
            this.copyButton.className = 'copy-button';
            this.copyButton.setAttribute('aria-label', 'Copy to clipboard');
            this.copyButton.innerHTML = this.getCopyButtonSVG();

            // Clear and populate output
            this.output.textContent = '';
            this.output.appendChild(this.contentDiv);
            this.output.appendChild(this.copyButton);

            // Set initial height
            this.output.style.minHeight = `${Math.max(
                this.output.getBoundingClientRect().height,
                TIMING.MIN_HEIGHT
            )}px`;
        }

        setupEventListeners() {
            this.resizeObserver = new ResizeObserver(
                this.throttle(this.updateHeight.bind(this), TIMING.RESIZE_THROTTLE)
            );
            this.resizeObserver.observe(this.contentDiv);
            this.copyButton.addEventListener('click', () => this.handleCopy());
        }

        getCopyButtonSVG(success = false) {
            return success ?
                `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3">
                    <path d="M9 11.286 10.8 13 15 9m-3-2.409-.154-.164c-1.978-2.096-5.249-1.85-6.927.522-1.489 2.106-1.132 5.085.806 6.729L12 19l6.275-5.322c1.938-1.645 2.295-4.623.806-6.729-1.678-2.372-4.949-2.618-6.927-.522z" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>` :
                `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3">
                    <path d="M10 8V7c0-.943 0-1.414.293-1.707S11.057 5 12 5h5c.943 0 1.414 0 1.707.293S19 6.057 19 7v5c0 .943 0 1.414-.293 1.707S17.943 14 17 14h-1m-9 5h5c.943 0 1.414 0 1.707-.293S14 17.943 14 17v-5c0-.943 0-1.414-.293-1.707S12.943 10 12 10H7c-.943 0-1.414 0-1.707.293S5 11.057 5 12v5c0 .943 0 1.414.293 1.707S6.057 19 7 19" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>`;
        }

        async handleCopy() {
            try {
                let content = this.contentDiv.textContent;

                if (!this.output.classList.contains('error') && /[▖▘▌]/.test(content)) {
                    content = content.replace(/[\n\r\t\u00A0\u2000-\u200C\u200E-\u200F\u2028-\u202F\uFEFF]/g, '');
                }

                await navigator.clipboard.writeText(content);
                this.copyButton.innerHTML = this.getCopyButtonSVG(true);
                setTimeout(() => {
                    this.copyButton.innerHTML = this.getCopyButtonSVG();
                }, TIMING.COPY_FEEDBACK_DURATION);
            } catch (err) {
                console.error('Copy failed:', err);
            }
        }

        clearErrorState() {
            if (this.state === State.ERROR) {
                this.output.classList.remove('error', 'shake');
                this.state = State.IDLE;
            }
        }

        async showError(error) {
            if (this.currentAnimation) {
                this.currentAnimation.cancel();
                this.currentAnimation = null;
            }

            this.chars = [];

            if (this.state !== State.ERROR) {
                this.state = State.ERROR;
                this.output.classList.remove('error', 'shake');
                void this.output.offsetWidth; // Force reflow
                this.output.classList.add('error', 'shake');

                setTimeout(() => {
                    this.output.classList.remove('shake');
                }, TIMING.SHAKE_DURATION);
            }

            this.contentDiv.textContent = error.toString();
            await this.updateHeight();
        }

        async setContent(content, immediate = false) {
            if (this.currentAnimation) {
                this.currentAnimation.cancel();
                this.currentAnimation = null;
            }

            this.clearErrorState();

            if (immediate) {
                this.chars = Array.from(content);
                this.contentDiv.textContent = content;
                await this.updateHeight();
                return;
            }

            const animation = {
                id: Symbol('animation'),
                cancelled: false,
                cancel() {
                    this.cancelled = true;
                }
            };
            this.currentAnimation = animation;

            try {
                const oldChars = [...this.chars];
                const newChars = Array.from(content);
                const commonLength = Math.min(oldChars.length, newChars.length);
                let i = 0;

                while (i < commonLength && oldChars[i] === newChars[i]) i++;

                if (oldChars.length > i) {
                    this.state = State.DELETING;
                    while (this.chars.length > i && !animation.cancelled) {
                        this.chars.pop();
                        this.contentDiv.textContent = this.chars.join('');
                        const speed = KeyState.isHoldThreshold('Backspace') ?
                            TIMING.DELETE_SPEED_FAST : TIMING.DELETE_SPEED;
                        await new Promise(r => setTimeout(r, speed));
                    }
                }

                if (!animation.cancelled) {
                    this.state = State.TYPING;
                    while (i < newChars.length && !animation.cancelled) {
                        this.chars.push(newChars[i++]);
                        this.contentDiv.textContent = this.chars.join('');
                        await new Promise(r => setTimeout(r, TIMING.CHAR_ANIMATION));
                    }
                }

            } finally {
                if (this.currentAnimation === animation) {
                    this.currentAnimation = null;
                    if (!animation.cancelled) {
                        this.state = State.IDLE;
                        this.chars = Array.from(content);
                        this.contentDiv.textContent = content;
                        await this.updateHeight();
                    }
                }
            }
        }

        async updateHeight() {
            const newHeight = this.contentDiv.scrollHeight;
            this.output.style.height = `${newHeight}px`;
        }

        throttle(func, limit) {
            let inThrottle;
            return function(...args) {
                if (!inThrottle) {
                    func.apply(this, args);
                    inThrottle = true;
                    setTimeout(() => inThrottle = false, limit);
                }
            };
        }

        calculateContentHeight(text) {
            const tempContent = document.createElement('div');
            tempContent.className = 'output-content';
            tempContent.style.visibility = 'hidden';
            tempContent.style.position = 'absolute';
            tempContent.style.width = this.contentDiv.offsetWidth + 'px';
            tempContent.textContent = text;
            this.output.appendChild(tempContent);
            const height = tempContent.scrollHeight;
            this.output.removeChild(tempContent);
            return height;
        }

        async processInput(value, immediate = false) {
            if (this.debounceTimeout) {
                clearTimeout(this.debounceTimeout);
                this.debounceTimeout = null;
            }

            if (!value.trim()) {
                if (this.currentAnimation) {
                    this.currentAnimation.cancel();
                    this.currentAnimation = null;
                }
                this.chars = [];
                this.clearErrorState();
                this.contentDiv.textContent = '';
                this.lastProcessedInput = '';
                await this.updateHeight();
                return;
            }

            if (value === this.lastProcessedInput) {
                return;
            }

            if (value.length > 50) {
                try {
                    const result = await this.wasm.convert(value);
                    const expectedHeight = this.calculateContentHeight(result);
                    this.output.style.height = `${expectedHeight}px`;
                } catch (err) {}
            }

            const process = async () => {
                if (this.isProcessing) return;

                try {
                    this.isProcessing = true;
                    const result = await this.wasm.convert(value);

                    if (value === this.lastProcessedInput) {
                        await this.setContent(result, immediate || value.length > 50);
                    }
                } catch (err) {
                    if (value === this.lastProcessedInput) {
                        console.error('Conversion error:', err);
                        await this.showError(err);
                    }
                } finally {
                    this.isProcessing = false;
                }
            };

            this.lastProcessedInput = value;

            if (immediate) {
                await process();
            } else {
                this.debounceTimeout = setTimeout(process, TIMING.DEBOUNCE_DELAY);
            }
        }

        cleanup() {
            if (this.currentAnimation) {
                this.currentAnimation.cancel();
                this.currentAnimation = null;
            }
            if (this.debounceTimeout) {
                clearTimeout(this.debounceTimeout);
                this.debounceTimeout = null;
            }
            this.resizeObserver?.disconnect();
            KeyState.clear();
            this.clearErrorState();
            this.isProcessing = false;
        }
    }

    function setupInfoPanel() {
        const infoButton = document.querySelector('#info-button');
        const infoPanel = document.querySelector('#info-panel');
        const closeButton = document.querySelector('.close-button');

        const showPanel = () => {
            infoPanel.hidden = false;
            infoPanel.setAttribute('aria-hidden', 'false');
            infoButton.setAttribute('aria-expanded', 'true');
            document.body.style.overflow = 'hidden';
        };

        const hidePanel = () => {
            infoPanel.hidden = true;
            infoPanel.setAttribute('aria-hidden', 'true');
            infoButton.setAttribute('aria-expanded', 'false');
            document.body.style.overflow = '';
        };

        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && !infoPanel.hidden) hidePanel();
        });

        infoButton.addEventListener('click', showPanel);
        closeButton.addEventListener('click', hidePanel);
        infoPanel.addEventListener('click', (e) => {
            if (e.target === infoPanel) hidePanel();
        });
    }

    async function loadWasm(retryCount = 0) {
        try {
            const { default: init, convert } = await import('./pkg/dollcode_wasm.js');
            await init();

            if (typeof convert !== 'function') {
                throw new Error('WASM convert function not found');
            }

            return { convert };
        } catch (error) {
            if (retryCount < TIMING.MAX_RETRIES) {
                await new Promise(r => setTimeout(r, TIMING.RETRY_DELAY * (retryCount + 1)));
                return loadWasm(retryCount + 1);
            }
            throw new Error(`WASM loading failed after ${TIMING.MAX_RETRIES} attempts: ${error.message}`);
        }
    }

    async function setupApp() {
        try {
            const input = document.querySelector("#input");
            if (input) {
                input.value = ''; // Clear input on page load/reload
            }

            const wasmModule = await loadWasm();

            document.documentElement.classList.remove('js-loading');

            const output = document.querySelector('#output');
            if (!output) throw new Error('Output element not found');

            const outputManager = new OutputManager(output, wasmModule);

            setupInfoPanel();

            if (!input) throw new Error('Input element not found');

            let inputBuffer = '';
            let isBuffering = false;
            let rafId = null;

            const flushBuffer = () => {
                if (inputBuffer) {
                    outputManager.processInput(inputBuffer, true);
                    inputBuffer = '';
                }
                isBuffering = false;
                rafId = null;
            };

            input.addEventListener('input', (e) => {
                const value = e.target.value.replace(/[\n\r\t\u00A0\u2000-\u200C\u200E-\u200F\u2028-\u202F\uFEFF]/g, '');

                if (value.length <= 1) {
                    outputManager.processInput(value, true);
                    return;
                }

                if (!isBuffering) {
                    isBuffering = true;
                    rafId = requestAnimationFrame(flushBuffer);
                }
                inputBuffer = value;
            });

            return () => {
                outputManager.cleanup();
                if (rafId) cancelAnimationFrame(rafId);
            };

        } catch (err) {
            console.error("Critical initialization error:", err);
            document.documentElement.classList.remove('js-loading');

            const output = document.querySelector('#output');
            if (output) {
                const errorMessage = err.message || 'Unknown error occurred';
                output.textContent = `Critical Error: ${errorMessage}`;
                output.classList.add('error');
            }
            throw err;
        }
    }

    try {
        await setupApp();
    } catch (err) {
        console.error("Fatal application error:", err);
        document.documentElement.classList.remove('js-loading');

        const output = document.querySelector('#output');
        if (output) {
            const errorMessage = err.message || 'Unknown error occurred';
            output.textContent = `Fatal Error: ${errorMessage}`;
            output.classList.add('error');
        }
    }
})();
