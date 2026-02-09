let app;
let lainModel;
let ws;
let audioContext;
let analyser;
let mouthLevel = 0;
let modelBaseSize;

const MODEL_PATH = "/static/models/lain/lain01_04.model.json";
const WS_URL = "ws://" + location.host + "/ws";

const statusEl = document.getElementById("status");
const captionEl = document.getElementById("caption");
const talkBtn = document.getElementById("talk-btn");

const setTalkButton = (label, disabled) => {
  talkBtn.textContent = label;
  talkBtn.disabled = disabled;
};

async function initLive2D() {
  if (!PIXI?.live2d?.Live2DModel?.from) {
    updateStatus("LIVE2D RUNTIME NOT LOADED");
    console.error(
      "Live2D runtime missing; check script order/CDN reachability.",
    );
    return;
  }

  app = new PIXI.Application({
    view: document.getElementById("screen"),
    autoStart: true,
    resizeTo: window,
    backgroundColor: 0x000000,
  });

  try {
    updateStatus("Loading model...");
    lainModel = await PIXI.live2d.Live2DModel.from(MODEL_PATH);
    app.stage.addChild(lainModel);

    resizeModel();

    window.addEventListener("resize", resizeModel);

    console.log("Lain Loaded");
    updateStatus("CONNECTED TO WIRED");
  } catch (error) {
    console.error("Error loading model:", error);
    updateStatus("ERROR LOADING MODEL");
  }
}

function resizeModel() {
  if (!lainModel) return;
  if (!app || !app.renderer) return;

  const screenW = app.renderer.width;
  const screenH = app.renderer.height;

  lainModel.scale.set(1, 1);

  const originalW = lainModel.width;
  const originalH = lainModel.height;

  const scaleByHeight = (screenH * 0.97) / originalH;
  const scaleByWidth = (screenW * 0.98) / originalW;
  const targetScale = Math.min(scaleByHeight, scaleByWidth);

  lainModel.scale.set(targetScale, targetScale);

  lainModel.x = (screenW - lainModel.width) / 2;
  lainModel.y = (screenH - lainModel.height) * 0.995;
}

function initWebSocket() {
  ws = new WebSocket(WS_URL);

  ws.onopen = () => {
    updateStatus("LINK ESTABLISHED");
    captionEl.innerText = "";
    setTalkButton("Start Talking", false);
  };
  ws.onclose = () => {
    updateStatus("LINK LOST");
    setTalkButton("Connecting...", true);
  };

  ws.onmessage = async (event) => {
    const data = JSON.parse(event.data);
    handleServerMessage(data);
  };
}

function handleServerMessage(data) {
  if (data.type === "status") {
    updateStatus(data.content);
  } else if (data.type === "caption") {
    captionEl.innerText = data.content;
  } else if (data.type === "audio") {
    updateStatus("SPEAKING...");
    playAudioWithLipSync("data:audio/mp3;base64," + data.content);
  } else if (data.type === "reset") {
    updateStatus("IDLE");
    setTalkButton("Start Talking", false);
  }
}

function playAudioWithLipSync(audioSrc) {
  if (!audioContext) {
    audioContext = new (window.AudioContext || window.webkitAudioContext)();
  }

  // 若因自动播放策略被挂起，尝试恢复
  if (audioContext.state === "suspended") {
    audioContext.resume();
  }

  const audio = new Audio(audioSrc);
  window.currentAudio = audio;

  if (!analyser) {
    analyser = audioContext.createAnalyser();
    analyser.fftSize = 1024;
    analyser.smoothingTimeConstant = 0.7;
  }

  const source = audioContext.createMediaElementSource(audio);
  source.connect(analyser);
  analyser.connect(audioContext.destination);

  audio.onended = () => {
    updateStatus("IDLE");
    setTalkButton("Start Talking", false);
    app.ticker.remove(lipSyncTicker);
    if (lainModel) {
      try {
        lainModel.internalModel.coreModel.setParamFloat(
          "PARAM_MOUTH_OPEN_Y",
          0,
        );
      } catch (e) {}
    }
  };

  audio.onerror = () => {
    updateStatus("AUDIO ERROR");
    setTalkButton("Start Talking", false);
    app.ticker.remove(lipSyncTicker);
  };

  app.ticker.add(lipSyncTicker);

  audio.play().catch((e) => {
    console.error("Play failed:", e);
    updateStatus("PLAYBACK FAILED");
    setTalkButton("Start Talking", false);
    app.ticker.remove(lipSyncTicker);
  });
}

const dataArray = new Uint8Array(1024);
const lipSyncTicker = () => {
  if (!window.currentAudio || window.currentAudio.paused) return;

  // 用时域 RMS + 峰值做能量，减少频域漂移
  analyser.getByteTimeDomainData(dataArray);
  let sumSq = 0;
  let peak = 0;
  for (let i = 0; i < dataArray.length; i++) {
    const centered = dataArray[i] - 128;
    sumSq += centered * centered;
    const abs = Math.abs(centered);
    if (abs > peak) peak = abs;
  }
  const rms = Math.sqrt(sumSq / dataArray.length) / 128; // 0~1
  const peakNorm = peak / 128; // 0~1
  const energy = rms * 0.7 + peakNorm * 0.3;

  // 噪声门与平滑，避免静音时张嘴
  const gate = 0.005;
  const target = energy > gate ? (energy - gate) * 4.0 : 0; // 提升灵敏度

  // 攻击/释放平滑
  const attack = 0.6;
  const release = 0.25;
  const alpha = target > mouthLevel ? attack : release;
  mouthLevel = mouthLevel * (1 - alpha) + target * alpha;

  const mouthValue = Math.min(mouthLevel, 1.0);

  if (lainModel) {
    try {
      lainModel.internalModel.coreModel.setParamFloat(
        "PARAM_MOUTH_OPEN_Y",
        mouthValue,
      );
    } catch (error) {}
  }
};

function updateStatus(text) {
  statusEl.innerText = "> " + text.toUpperCase();
}

talkBtn.addEventListener("click", () => {
  if (ws && ws.readyState === WebSocket.OPEN) {
    updateStatus("RECORDING...");
    setTalkButton("Recording...", true);
    ws.send("start_interaction");

    if (audioContext && audioContext.state === "suspended") {
      audioContext.resume();
    }
  }
});

function loadScript(url) {
  return new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = url;
    s.async = true;
    s.onload = () => resolve(url);
    s.onerror = () => reject(new Error("Failed to load " + url));
    document.head.appendChild(s);
  });
}

async function loadWithFallbacks(candidates, label) {
  for (const url of candidates) {
    try {
      await loadScript(url);
      console.log(`Loaded ${label} from ${url}`);
      return;
    } catch (err) {
      console.warn(err.message);
    }
  }
  throw new Error(`All sources failed for ${label}`);
}

async function bootstrap() {
  try {
    await loadWithFallbacks(
      [
        "/static/js/vendor/pixi.min.js",
        "https://cdn.jsdelivr.net/npm/pixi.js@7.x/dist/pixi.min.js",
      ],
      "PixiJS",
    );
    const live2dOk = await loadWithFallbacks(
      [
        "/static/js/vendor/live2d.min.js",
        "https://cdn.jsdelivr.net/gh/dizhenry/Live2D_v2/live2d.min.js",
      ],
      "Live2D Cubism2 runtime",
    ).then(
      () => true,
      (err) => {
        console.warn(err.message);
        return false;
      },
    );

    if (live2dOk) {
      await loadWithFallbacks(
        [
          "/static/js/vendor/pixi-live2d-display-cubism2.min.js",
          "https://cdn.jsdelivr.net/npm/pixi-live2d-display/dist/cubism2.min.js",
        ],
        "pixi-live2d-display cubism2 bundle",
      );
      initLive2D();
    } else {
      updateStatus("LIVE2D RUNTIME MISSING (put live2d.min.js in static/js/vendor/)");
    }
  } catch (error) {
    updateStatus("LIVE2D DEPENDENCIES UNAVAILABLE");
    console.error(error);
  } finally {
    initWebSocket();
  }
}

window.addEventListener("load", bootstrap);
