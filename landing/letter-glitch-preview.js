const previewRoot = document.getElementById("letter-glitch-preview");

if (previewRoot) {
  const canvas = document.createElement("canvas");
  previewRoot.appendChild(canvas);

  const ctx = canvas.getContext("2d");
  const glitchColors = ["#2b4539", "#61dca3", "#61b3dc"];
  const characters = Array.from(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ!@#$&*()-_+=/[]{};:<>.,0123456789"
  );
  const fontSize = 16;
  const charWidth = 10;
  const charHeight = 20;
  const letters = [];
  let columns = 0;
  let rows = 0;
  let lastGlitchTime = Date.now();

  const randomItem = (items) => items[Math.floor(Math.random() * items.length)];

  const hexToRgb = (hex) => {
    const normalized = hex.replace(
      /^#?([a-f\d])([a-f\d])([a-f\d])$/i,
      (_, r, g, b) => `#${r}${r}${g}${g}${b}${b}`
    );
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(normalized);
    return result
      ? {
          r: parseInt(result[1], 16),
          g: parseInt(result[2], 16),
          b: parseInt(result[3], 16),
        }
      : null;
  };

  const interpolateColor = (start, end, factor) => {
    const r = Math.round(start.r + (end.r - start.r) * factor);
    const g = Math.round(start.g + (end.g - start.g) * factor);
    const b = Math.round(start.b + (end.b - start.b) * factor);
    return `rgb(${r}, ${g}, ${b})`;
  };

  const initializeLetters = () => {
    letters.length = 0;
    for (let i = 0; i < columns * rows; i += 1) {
      const color = randomItem(glitchColors);
      letters.push({
        char: randomItem(characters),
        color,
        baseColor: color,
        targetColor: randomItem(glitchColors),
        colorProgress: 1,
      });
    }
  };

  const resize = () => {
    const rect = previewRoot.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;

    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    columns = Math.ceil(rect.width / charWidth);
    rows = Math.ceil(rect.height / charHeight);
    initializeLetters();
    draw();
  };

  const draw = () => {
    const rect = previewRoot.getBoundingClientRect();
    ctx.clearRect(0, 0, rect.width, rect.height);
    ctx.font = `${fontSize}px IBM Plex Mono, monospace`;
    ctx.textBaseline = "top";

    letters.forEach((letter, index) => {
      const x = (index % columns) * charWidth;
      const y = Math.floor(index / columns) * charHeight;
      ctx.fillStyle = letter.color;
      ctx.fillText(letter.char, x, y);
    });

    const vignette = ctx.createRadialGradient(
      rect.width / 2,
      rect.height / 2,
      rect.width * 0.08,
      rect.width / 2,
      rect.height / 2,
      rect.width * 0.7
    );
    vignette.addColorStop(0, "rgba(0,0,0,0)");
    vignette.addColorStop(1, "rgba(0,0,0,0.92)");
    ctx.fillStyle = vignette;
    ctx.fillRect(0, 0, rect.width, rect.height);
  };

  const updateLetters = () => {
    const updateCount = Math.max(1, Math.floor(letters.length * 0.05));
    for (let i = 0; i < updateCount; i += 1) {
      const index = Math.floor(Math.random() * letters.length);
      const nextColor = randomItem(glitchColors);
      letters[index].char = randomItem(characters);
      letters[index].baseColor = letters[index].color;
      letters[index].targetColor = nextColor;
      letters[index].colorProgress = 0;
    }
  };

  const animate = () => {
    const now = Date.now();

    if (now - lastGlitchTime >= 50) {
      updateLetters();
      lastGlitchTime = now;
    }

    let shouldRedraw = false;
    letters.forEach((letter) => {
      if (letter.colorProgress < 1) {
        const start = hexToRgb(letter.baseColor) || hexToRgb("#61dca3");
        const end = hexToRgb(letter.targetColor) || hexToRgb("#61b3dc");
        letter.colorProgress = Math.min(1, letter.colorProgress + 0.06);
        letter.color = interpolateColor(start, end, letter.colorProgress);
        shouldRedraw = true;
      }
    });

    if (shouldRedraw) {
      draw();
    }

    requestAnimationFrame(animate);
  };

  document.querySelectorAll("[data-copy-target]").forEach((button) => {
    button.addEventListener("click", async () => {
      const target = document.getElementById(button.dataset.copyTarget);
      if (!target) return;
      try {
        await navigator.clipboard.writeText(target.innerText);
        button.textContent = "Copied";
        setTimeout(() => {
          button.textContent = "Copy";
        }, 1200);
      } catch (_error) {
        button.textContent = "Failed";
      }
    });
  });

  window.addEventListener("resize", resize);
  resize();
  animate();
}
