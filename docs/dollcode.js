(async function () {
    try {
        // Load stylesheets first
        await Promise.all(
            Array.from(document.querySelectorAll('link[rel="stylesheet"]'))
                .map(link => new Promise((resolve) => {
                    if (link.sheet) {
                        try {
                            const _ = link.sheet.cssRules;
                            resolve();
                        } catch {
                            link.addEventListener('load', resolve);
                            link.addEventListener('error', resolve);
                        }
                    } else {
                        link.addEventListener('load', resolve);
                        link.addEventListener('error', resolve);
                    }
                }))
        );

        // Then load WASM
        const {
            default: init,
            convert_decimal,
            convert_hex,
            convert_text,
            convert_dollcode
        } = await import('./pkg/dollcode_wasm.js');

        await init();
        document.documentElement.classList.remove('js-loading');

        // Info panel handling
        const infoButton = document.querySelector('#info-button');
        const infoPanel = document.querySelector('#info-panel');
        const closeButton = document.querySelector('.close-button');

        function showInfoPanel() {
            infoPanel.hidden = false;
            infoPanel.setAttribute('aria-hidden', 'false');
            infoButton.setAttribute('aria-expanded', 'true');
            document.body.style.overflow = 'hidden';
        }

        function hideInfoPanel() {
            infoPanel.hidden = true;
            infoPanel.setAttribute('aria-hidden', 'true');
            infoButton.setAttribute('aria-expanded', 'false');
            document.body.style.overflow = '';
        }

        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && !infoPanel.hidden) {
                hideInfoPanel();
            }
        });

        infoButton.addEventListener('click', showInfoPanel);
        closeButton.addEventListener('click', hideInfoPanel);
        infoPanel.addEventListener('click', (e) => {
            if (e.target === infoPanel) hideInfoPanel();
        });

        // Setup output structure
        const output = document.querySelector('#output');

        // Create content div and move existing content into it
        const contentDiv = document.createElement('div');
        contentDiv.className = 'output-content';
        contentDiv.textContent = output.textContent.trim(); // Preserve initial content
        output.textContent = ''; // Clear the output
        output.appendChild(contentDiv);

        // Create copy button
        const copyButton = document.createElement('button');
        copyButton.className = 'copy-button';
        copyButton.setAttribute('aria-label', 'Copy to clipboard');
        copyButton.innerHTML = `
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
            </svg>
        `;

        copyButton.addEventListener('click', async () => {
            try {
                await navigator.clipboard.writeText(contentDiv.textContent.trim());

                copyButton.innerHTML = `
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="20 6 9 17 4 12"/>
                    </svg>
                `;

                setTimeout(() => {
                    copyButton.innerHTML = `
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                        </svg>
                    `;
                }, 2000);
            } catch (err) {
                console.error('Failed to copy:', err);
            }
        });

        output.appendChild(copyButton);

        function setOutput(content) {
            const output = document.querySelector("#output");
            const contentDiv = output.querySelector('.output-content');

            output.style.removeProperty('color');
            output.classList.remove('error', 'shake');

            if (!(content instanceof Error) && !(typeof content === 'string' && content.includes('Error'))) {
                output.classList.add('transitioning');
            }

            requestAnimationFrame(() => {
                if (content instanceof Error || (typeof content === 'string' && content.includes('Error'))) {
                    const msg = content.toString();
                    let displayMessage;

                    if (msg.includes('exceeds maximum') || msg.includes('Overflow') ||
                        msg.includes('Buffer') || msg.includes('Input exceeds maximum supported size')) {
                        displayMessage = 'Input limit exceeded';
                    } else if (msg.includes('Invalid char') || msg.includes('Invalid character')) {
                        displayMessage = msg.includes('emoji') ?
                            'Invalid character detected: emoji' :
                            'Invalid character detected';
                    } else {
                        displayMessage = 'Conversion error occurred';
                    }

                    output.classList.add('error');
                    contentDiv.textContent = displayMessage;
                    requestAnimationFrame(() => {
                        output.classList.add('shake');
                    });
                    return;
                }

                if (typeof content === 'string' && content.startsWith('d:')) {
                    const [dec, hex] = content.split(',');
                    contentDiv.innerHTML = `<div>${dec.trim()}</div><div>${hex.trim()}</div>`;
                } else {
                    contentDiv.textContent = content ? content.toString().trim() : '';
                }

                requestAnimationFrame(() => {
                    output.classList.remove('transitioning');
                });
            });
        }

        function detectInputType(value) {
            if (!value.trim()) return null;

            // Check for script injection patterns
            const securityCheck = /<[^>]*script|javascript:|data:|vbscript:|on\w+\s*=/i;
            if (securityCheck.test(value)) {
                throw new Error('Invalid input detected');
            }

            if (/^[▖▘▌]+$/.test(value)) return 'dollcode';
            if (/^0x[0-9A-Fa-f]+$/i.test(value)) return 'hex';
            if (/^\d+$/.test(value)) return 'decimal';

            // Check for emoji
            const emojiCheck = /[\p{Emoji_Presentation}\p{Extended_Pictographic}]/u;
            if (emojiCheck.test(value)) {
                throw new Error('Invalid character detected: emoji');
            }

            return 'text';
        }

        // Input handling with debouncing
        const input = document.querySelector("#input");
        let debounceTimeout;

        input.addEventListener('input', (e) => {
            clearTimeout(debounceTimeout);
            debounceTimeout = setTimeout(async () => {
                try {
                    const inputType = detectInputType(e.target.value);
                    if (!inputType) {
                        setOutput('');
                        return;
                    }

                    let result;
                    switch (inputType) {
                        case 'dollcode':
                            result = await convert_dollcode(e.target.value);
                            break;
                        case 'hex':
                            result = await convert_hex(e.target.value);
                            break;
                        case 'text':
                            result = await convert_text(e.target.value);
                            break;
                        case 'decimal':
                        default:
                            result = await convert_decimal(e.target.value);
                    }
                    setOutput(result);
                } catch (err) {
                    console.error(err);
                    setOutput(err);
                }
            }, 150);
        });

        // Initial conversion
        const initialInput = input.value;
        if (initialInput) {
            try {
                const result = await convert_decimal(initialInput);
                setOutput(result);
            } catch (err) {
                console.error(err);
                setOutput(err);
            }
        }

    } catch (err) {
        console.error("Failed to initialize WASM:", err);
        document.querySelector("#output").textContent = "Error: Failed to load dollcode";
        document.body.classList.remove('js-loading');
    }
})();
