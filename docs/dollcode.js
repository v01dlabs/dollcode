(async function() {
    const TIMING = {
        ANIMATION: {
            CHAR: 4,
            DELETE: 8,
            DELETE_FAST: 4,
            SHAKE: 400,
            COPY_FEEDBACK: 1200,
            COPY_FADE: 300,
            COPY_COOLDOWN: 400,
            LOADING: 600,
            LOADING_CLEANUP: 1200
        },
        PHYSICS: {
            VELOCITY_FACTOR: 0.1,
            MIN_VELOCITY: 0,
            MAX_VELOCITY: 10,
            FADE_DISTANCE: 5
        },
        PERFORMANCE: {
            DEBOUNCE: 20,
            RESIZE_THROTTLE: 20,
            RAF_TIMEOUT: 80
        },
        LOADING: {
            MAX_RETRIES: 5,
            RETRY_DELAY: 200,
            TIMEOUT: 10000
        },
        INPUT: {
            HELD_THRESHOLD: 300
        }
    };

    const createLoadingAnimation = (outputElement, finalText = "", options = {}) => {
        const {
            duration = TIMING.ANIMATION.LOADING,
            fps = 60,
            minChars = 32,
            maxChars = 126
        } = options;

        const frameTime = 2400 / fps;
        let startTime = null;
        let lastFrameTime = null;
        let animationFrame = null;

        const getRandomChar = () => String.fromCharCode(
            minChars + Math.floor(Math.random() * (maxChars - minChars + 1))
        );

        const getGlitchText = (target) => {
            const tempSpan = document.createElement('span');
            tempSpan.style.visibility = 'hidden';
            tempSpan.style.whiteSpace = 'nowrap';
            outputElement.appendChild(tempSpan);

            const computedStyle = window.getComputedStyle(outputElement);
            const paddingLeft = parseFloat(computedStyle.paddingLeft);
            const availableWidth = outputElement.clientWidth - paddingLeft + 42;

            let chars = '';
            while (tempSpan.offsetWidth < availableWidth) {
                chars += getRandomChar();
                tempSpan.textContent = chars;
            }

            tempSpan.textContent = chars;
            if (tempSpan.offsetWidth > (availableWidth)) {
                chars = chars.slice(0, -1);
            }

            outputElement.removeChild(tempSpan);

            return chars.split('').map((char, i) =>
                target[i] === ' ' || target[i] === '\u200D' ? target[i] : char
            ).join('');
        };

        const animate = (timestamp) => {
            if (!startTime) startTime = timestamp;
            if (!lastFrameTime) lastFrameTime = timestamp;

            const progress = timestamp - startTime;
            const delta = timestamp - lastFrameTime;

            if (delta >= frameTime) {
                lastFrameTime = timestamp;
                outputElement.classList.add('expanding');

                if (progress < duration) {
                    outputElement.textContent = getGlitchText(finalText);
                } else {
                    outputElement.textContent = finalText;
                    outputElement.classList.remove('expanding');
                    animationFrame = null;
                    return;
                }
            }

            animationFrame = requestAnimationFrame(animate);
        };

        const start = () => {
            if (animationFrame) cancelAnimationFrame(animationFrame);
            startTime = null;
            lastFrameTime = null;
            animationFrame = requestAnimationFrame(animate);
        };

        const stop = () => {
            if (animationFrame) {
                cancelAnimationFrame(animationFrame);
                animationFrame = null;
            }
            outputElement.textContent = finalText;
            outputElement.classList.remove('expanding');
        };

        const destroy = () => {
            stop();
            startTime = null;
            lastFrameTime = null;
        };

        return { start, stop, destroy };
    };

    const State = Object.freeze({
        IDLE: 'idle',
        TYPING: 'typing',
        DELETING: 'deleting',
        ERROR: 'error'
    });

    const SELECTORS = Object.freeze({
        INPUT: '#input',
        OUTPUT: '#output',
        INFO_BUTTON: '#info-button',
        INFO_PANEL: '#info-panel',
        CLOSE_BUTTON: '.close-button'
    });

    const CLASSES = Object.freeze({
        CONTENT: 'output-content',
        ERROR: 'error',
        SHAKE: 'shake'
    });

    const keyState = new WeakMap();

    class KeyStateManager {
        constructor() {
            keyState.set(this, new Map());
        }

        isKeyHeld(key) {
            return keyState.get(this).has(key);
        }

        getHoldTime(key) {
            const startTime = keyState.get(this).get(key);
            return startTime ? Date.now() - startTime : 0;
        }

        isHoldThreshold(key) {
            return this.getHoldTime(key) > TIMING.INPUT.HELD_THRESHOLD;
        }

        pressKey(key) {
            if (!this.isKeyHeld(key)) {
                keyState.get(this).set(key, Date.now());
            }
        }

        releaseKey(key) {
            keyState.get(this).delete(key);
        }

        clear() {
            keyState.get(this).clear();
        }
    }

    class OutputManager {
        #state;
        #output;
        #contentDiv;
        #chars;
        #lastInput;
        #currentAnimation;
        #debounceTimeout;
        #isProcessing;
        #resizeObserver;
        #wasm;
        #keyState;
        #heightRAF = null;
        #lastHeight = 0;

        constructor(outputElement, wasmFunctions) {
            this.#state = State.IDLE;
            this.#output = outputElement;
            this.#wasm = wasmFunctions;
            this.#chars = [];
            this.#lastInput = '';
            this.#currentAnimation = null;
            this.#debounceTimeout = null;
            this.#isProcessing = false;
            this.#keyState = new KeyStateManager();

            this.#setupDOM();
            this.#setupEventListeners();
            this.#setupCopyFeedback();
        }

        #setupDOM() {
            this.#output.style.visibility = 'hidden';
            const fragment = document.createDocumentFragment();

            this.#contentDiv = document.createElement('div');
            this.#contentDiv.className = CLASSES.CONTENT;
            this.#contentDiv.style.cssText = 'transform: translateZ(0); will-change: transform;';

            const initialText = '▖▌▘▘‍▖▌▖▘‍▖▘▖▌‍';
            const loadingAnim = createLoadingAnimation(this.#contentDiv, initialText);
            loadingAnim.start();

            fragment.appendChild(this.#contentDiv);
            this.#output.textContent = '';
            this.#output.appendChild(fragment);

            requestAnimationFrame(() => {
                this.#output.style.visibility = 'visible';
                setTimeout(() => loadingAnim.destroy(), TIMING.ANIMATION.LOADING_CLEANUP);
            });
        }

        #setupEventListeners() {
            this.#resizeObserver = new ResizeObserver(
                this.#throttle(this.#updateHeight.bind(this), TIMING.PERFORMANCE.RESIZE_THROTTLE)
            );
            this.#resizeObserver.observe(this.#contentDiv);

            const observer = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        this.#contentDiv.style.willChange = 'transform, opacity';
                    } else {
                        this.#contentDiv.style.willChange = 'auto';
                    }
                });
            });

            observer.observe(this.#output);
        }

        #setupCopyFeedback() {
            let hoverFeedback = null;
            let isHovering = false;
            let copyAnimationInProgress = false;
            let copyAnimationTimeout = null;

            const createCopyFeedback = (e) => {
                const feedback = document.createElement('div');
                feedback.className = 'copy-feedback';

                feedback.style.left = `${e.clientX + 12}px`;
                feedback.style.top = `${e.clientY - 12}px`;

                feedback.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3">
                    <path d="M9 11.286 10.8 13 15 9m-3-2.409-.154-.164c-1.978-2.096-5.249-1.85-6.927.522-1.489 2.106-1.132 5.085.806 6.729L12 19l6.275-5.322c1.938-1.645 2.295-4.623.806-6.729-1.678-2.372-4.949-2.618-6.927-.522z" stroke-linecap="round" stroke-linejoin="round"/>`;

                document.body.appendChild(feedback);

                const angleRange = 60;
                const randomAngle = (Math.random() * angleRange - angleRange/2) * (Math.PI / 180);

                const baseAngle = -Math.PI/2;
                const finalAngle = baseAngle + randomAngle;

                const velocity = 60;
                const directionX = Math.cos(finalAngle) * velocity;
                const directionY = Math.sin(finalAngle) * velocity;

                const rotation = (Math.random() * 60 - 30);

                const waveAmplitude = 30;
                const waveX = Math.cos(finalAngle + Math.PI/2) * waveAmplitude;
                const waveY = Math.sin(finalAngle + Math.PI/2) * waveAmplitude;

                feedback.style.setProperty('--move-x', `${directionX}px`);
                feedback.style.setProperty('--move-y', `${directionY}px`);
                feedback.style.setProperty('--rotation', `${rotation}deg`);
                feedback.style.setProperty('--wave-x', `${waveX}px`);
                feedback.style.setProperty('--wave-y', `${waveY}px`);

                return feedback;
            };

            const showHoverFeedback = (e) => {
                if (copyAnimationInProgress) return;

                if (!hoverFeedback) {
                    hoverFeedback = document.createElement('div');
                    hoverFeedback.className = 'hover-feedback';
                    hoverFeedback.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3">
                        <path d="M10 8V7c0-.943 0-1.414.293-1.707S11.057 5 12 5h5c.943 0 1.414 0 1.707.293S19 6.057 19 7v5c0 .943 0 1.414-.293 1.707S17.943 14 17 14h-1m-9 5h5c.943 0 1.414 0 1.707-.293S14 17.943 14 17v-5c0-.943 0-1.414-.293-1.707S12.943 10 12 10H7c-.943 0-1.414 0-1.707.293S5 11.057 5 12v5c0 .943 0 1.414.293 1.707S6.057 19 7 19" stroke-linecap="round" stroke-linejoin="round"/>`;
                    document.body.appendChild(hoverFeedback);

                    void hoverFeedback.offsetHeight;
                }

                hoverFeedback.style.left = `${e.clientX + 12}px`;
                hoverFeedback.style.top = `${e.clientY - 12}px`;
                requestAnimationFrame(() => {
                    hoverFeedback.style.opacity = '1';
                });
            };

            this.#contentDiv.addEventListener('mousemove', (e) => {
                isHovering = true;
                if (!copyAnimationInProgress) {
                    showHoverFeedback(e);
                }
            });

            this.#contentDiv.addEventListener('mouseleave', () => {
                isHovering = false;
                if (hoverFeedback) {
                    hoverFeedback.style.opacity = '0';
                    setTimeout(() => {
                        if (!isHovering && hoverFeedback) {
                            hoverFeedback.remove();
                            hoverFeedback = null;
                        }
                    }, TIMING.ANIMATION.COPY_FADE);
                }
            });

            this.#contentDiv.addEventListener('click', async (e) => {
                try {
                    if (copyAnimationTimeout) {
                        clearTimeout(copyAnimationTimeout);
                    }

                    copyAnimationInProgress = true;

                    if (hoverFeedback) {
                        hoverFeedback.style.opacity = '0';
                        setTimeout(() => {
                            if (hoverFeedback) {
                                hoverFeedback.remove();
                                hoverFeedback = null;
                            }
                        }, TIMING.ANIMATION.COPY_FADE);
                    }

                    await navigator.clipboard.writeText(this.#contentDiv.textContent);
                    const copyFeedback = createCopyFeedback(e);

                    copyFeedback.classList.add('active');

                    copyFeedback.addEventListener('animationend', () => {
                        copyFeedback.classList.add('fading');

                        setTimeout(() => {
                            copyFeedback.remove();
                            copyAnimationTimeout = setTimeout(() => {
                                copyAnimationInProgress = false;
                            }, TIMING.ANIMATION.COPY_COOLDOWN);
                        }, TIMING.ANIMATION.COPY_FADE);
                    });
                } catch (err) {
                    console.error('Failed to copy:', err);
                    copyAnimationInProgress = false;
                }
            });
        }

        #clearErrorState() {
            if (this.#state === State.ERROR) {
                this.#output.classList.remove(CLASSES.ERROR, CLASSES.SHAKE);
                this.#state = State.IDLE;
            }
        }

        async #showError(error) {
            if (this.#currentAnimation) {
                this.#currentAnimation.cancel();
                this.#currentAnimation = null;
            }

            this.#chars = [];

            if (this.#state !== State.ERROR) {
                this.#state = State.ERROR;
                this.#output.classList.remove(CLASSES.ERROR, CLASSES.SHAKE);
                void this.#output.offsetWidth; // Force reflow
                this.#output.classList.add(CLASSES.ERROR, CLASSES.SHAKE);

                setTimeout(() => {
                    this.#output.classList.remove(CLASSES.SHAKE);
                }, TIMING.ANIMATION.SHAKE);
            }

            requestAnimationFrame(() => {
                this.#contentDiv.textContent = error.toString();
                this.#updateHeight();
            });
        }

        async #setContent(content, immediate = false) {
            if (this.#currentAnimation) {
                this.#currentAnimation.cancel();
                this.#currentAnimation = null;
            }

            this.#clearErrorState();

            if (immediate) {
                this.#chars = Array.from(content);
                this.#contentDiv.textContent = content;
                this.#updateHeight();
                return;
            }

            const animation = {
                id: Symbol('animation'),
                cancelled: false,
                cancel() { this.cancelled = true; }
            };
            this.#currentAnimation = animation;

            try {
                const oldChars = [...this.#chars];
                const newChars = Array.from(content);
                let i = 0;

                while (i < oldChars.length && i < newChars.length && oldChars[i] === newChars[i]) i++;

                const updateContent = () => {
                    if (!animation.cancelled) {
                        this.#contentDiv.textContent = this.#chars.join('');
                    }
                };

                if (oldChars.length > i) {
                    this.#state = State.DELETING;
                    while (this.#chars.length > i && !animation.cancelled) {
                        this.#chars.pop();
                        requestAnimationFrame(updateContent);
                        const speed = this.#keyState.isHoldThreshold('Backspace') ?
                            TIMING.ANIMATION.DELETE_FAST : TIMING.ANIMATION.DELETE;
                        await new Promise(r => setTimeout(r, speed));
                    }
                }

                if (!animation.cancelled) {
                    this.#state = State.TYPING;
                    while (i < newChars.length && !animation.cancelled) {
                        this.#chars.push(newChars[i++]);
                        requestAnimationFrame(updateContent);
                        await new Promise(r => setTimeout(r, TIMING.ANIMATION.CHAR));
                    }
                }

                if (!animation.cancelled) {
                    this.#state = State.IDLE;
                    this.#chars = Array.from(content);
                    this.#contentDiv.textContent = content;
                    this.#updateHeight();
                }
            } finally {
                if (this.#currentAnimation === animation) {
                    this.#currentAnimation = null;
                }
            }
        }

        #updateHeight = () => {
            if (!this.#heightRAF) {
                this.#heightRAF = requestAnimationFrame(() => {
                    const computedStyle = window.getComputedStyle(this.#output);
                    const minHeight = parseFloat(computedStyle.minHeight);
                    const padding = parseFloat(computedStyle.paddingTop) + parseFloat(computedStyle.paddingBottom);
                    const contentHeight = this.#contentDiv.scrollHeight + padding;

                    const newHeight = Math.max(contentHeight, minHeight);

                    if (newHeight !== this.#lastHeight) {
                        this.#output.style.height = `${newHeight}px`;
                        this.#lastHeight = newHeight;
                    }

                    this.#heightRAF = null;
                });
            }
        }

        #throttle(func, limit) {
            let inThrottle;
            return function(...args) {
                if (!inThrottle) {
                    func.apply(this, args);
                    inThrottle = true;
                    setTimeout(() => inThrottle = false, limit);
                }
            };
        }

        async processInput(value, immediate = false) {
            if (this.#debounceTimeout) {
                clearTimeout(this.#debounceTimeout);
                this.#debounceTimeout = null;
            }

            if (!value.trim()) {
                if (this.#currentAnimation) {
                    this.#currentAnimation.cancel();
                    this.#currentAnimation = null;
                }
                this.#chars = [];
                this.#clearErrorState();
                requestAnimationFrame(() => {
                    this.#contentDiv.textContent = '';
                    this.#updateHeight();
                });
                this.#lastInput = '';
                return;
            }

            if (value === this.#lastInput) return;

            const process = async () => {
                if (this.#isProcessing) return;

                try {
                    this.#isProcessing = true;
                    const result = await this.#wasm.convert(value);

                    if (value === this.#lastInput) {
                        await this.#setContent(result, immediate || value.length > 50);
                    }
                } catch (err) {
                    if (value === this.#lastInput) {
                        console.error('Conversion error:', err);
                        await this.#showError(err);
                    }
                } finally {
                    this.#isProcessing = false;
                }
            };

            this.#lastInput = value;

            if (immediate) {
                await process();
            } else {
                this.#debounceTimeout = setTimeout(process, TIMING.PERFORMANCE.DEBOUNCE);
            }
        }

        cleanup() {
            if (this.#currentAnimation) {
                this.#currentAnimation.cancel();
                this.#currentAnimation = null;
            }
            if (this.#debounceTimeout) {
                clearTimeout(this.#debounceTimeout);
                this.#debounceTimeout = null;
            }
            this.#resizeObserver?.disconnect();
            this.#keyState.clear();
            this.#clearErrorState();
            this.#isProcessing = false;
        }
    }

    function setupInfoPanel() {
        const infoButton = document.querySelector(SELECTORS.INFO_BUTTON);
        const infoPanel = document.querySelector(SELECTORS.INFO_PANEL);
        const closeButton = document.querySelector(SELECTORS.CLOSE_BUTTON);

        if (!infoButton || !infoPanel || !closeButton) return;

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

        const handleKeydown = (e) => {
            if (e.key === 'Escape' && !infoPanel.hidden) {
                hidePanel();
            }
        };

        document.addEventListener('keydown', handleKeydown);
        infoButton.addEventListener('click', showPanel);
        closeButton.addEventListener('click', hidePanel);

        infoPanel.addEventListener('click', (e) => {
            if (e.target === infoPanel) hidePanel();
        });

        return () => {
            document.removeEventListener('keydown', handleKeydown);
        };
    }

    async function loadWasm(retryCount = 0) {
        try {
            console.log('Attempting WASM load, retry:', retryCount);

            if (!('WebAssembly' in window)) {
                throw new Error('WebAssembly is not supported in this browser');
            }

            try {
                await WebAssembly.instantiate(new WebAssembly.Module(new Uint8Array([
                    0x0, 0x61, 0x73, 0x6d, 0x1, 0x0, 0x0, 0x0
                ])));
            } catch (e) {
                throw new Error('JavaScript JIT is required for WebAssembly support.\nTry disabling private browsing or content blockers.');
            }

            const wasmCheck = await fetch('./pkg/dollcode_wasm_bg.wasm');
            if (!wasmCheck.ok) {
                throw new Error(`WASM file not found: ${wasmCheck.status}`);
            }
            console.log('WASM file found, attempting module import');

            const { default: init, convert } = await import('./pkg/dollcode_wasm.js');
            console.log('Module imported, initializing...');

            await init();
            console.log('WASM initialized');

            if (typeof convert !== 'function') {
                throw new Error('WASM convert function not found');
            }

            return { convert };
        } catch (error) {
            console.error('WASM load error:', error);
            if (retryCount < TIMING.LOADING.MAX_RETRIES) {
                const delay = TIMING.LOADING.RETRY_DELAY * (retryCount + 1);
                console.log(`Retrying in ${delay}ms...`);
                await new Promise(r => setTimeout(r, delay));
                return loadWasm(retryCount + 1);
            }
            throw error;
        }
    }

    async function setupApp() {
        try {
            const input = document.querySelector(SELECTORS.INPUT);
            const output = document.querySelector(SELECTORS.OUTPUT);

            if (!input || !output) {
                throw new Error('Required elements not found');
            }

            input.value = '';

            const wasmModule = await Promise.race([
                loadWasm(),
                new Promise((_, reject) =>
                    setTimeout(() => reject(new Error('WASM load timeout')),
                    TIMING.LOADING.TIMEOUT)
                )
            ]);

            const outputManager = new OutputManager(output, wasmModule);
            const cleanupInfoPanel = setupInfoPanel();

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

            const handleInput = (e) => {
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
            };

            input.addEventListener('input', handleInput, { passive: true });

            return () => {
                outputManager.cleanup();
                cleanupInfoPanel();
                input.removeEventListener('input', handleInput);
                if (rafId) cancelAnimationFrame(rafId);
            };

        } catch (err) {
            console.error("Critical initialization error:", err);
            document.documentElement.classList.remove('js');

            const output = document.querySelector(SELECTORS.OUTPUT);
            if (output) {
                const errorMessage = err.message || 'Unknown error occurred';
                output.textContent = `Critical Error: ${errorMessage}`;
                output.classList.add(CLASSES.ERROR);
                output.setAttribute('aria-live', 'assertive');
            }
            throw err;
        }
    }

    try {
        const cleanup = await setupApp();
        window.addEventListener('pagehide', () => {
            cleanup();
        }, { passive: true });
    } catch (err) {
        console.error("Fatal application error:", err);
        const output = document.querySelector(SELECTORS.OUTPUT);
        if (output) {
            const errorMessage = err.message || 'Unknown error occurred';
            output.textContent = `Fatal Error: ${errorMessage}`;
            output.classList.add(CLASSES.ERROR);
            output.setAttribute('aria-live', 'assertive');
        }
    }
})();
