let app;
let lainModel;
let ws;
let audioContext;
let analyser;

const MODEL_PATH = "/static/models/lain/lain01_04.model.json";
const WS_URL = "ws://" + location.host + "/ws";

const statusEl = document.getElementById("status");
const captionEl = document.getElementById("caption");
const talkBtn = document.getElementById("talk-btn");

async function initLive2D() {
  if (!PIXI?.live2d?.Live2DModel?.from) {
    updateStatus("LIVE2D RUNTIME NOT LOADED");
    console.error("Live2D runtime missing; check script order/CDN reachability.");
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
  const scaleX = (window.innerWidth * 0.5) / lainModel.width;
  const scaleY = (window.innerHeight * 0.8) / lainModel.height;
  const scale = Math.min(scaleX, scaleY);

  lainModel.scale.set(scale);
  lainModel.x = (window.innerWidth - lainModel.width * scale) / 2;
  lainModel.y = (window.innerHeight - lainModel.height) / 2 + 50;
}

function initWebSocket() {
  ws = new WebSocket(WS_URL);

  ws.onopen = () => {
    updateStatus("LINK ESTABLISHED");
    talkBtn.disabled = false;
  };
  ws.onclose = () => {
    updateStatus("LINK LOST");
    talkBtn.disabled = true;
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
    // server sends base64 in `content`
    playAudioWithLipSync("data:audio/mp3;base64," + data.content);
  }
}

function playAudioWithLipSync(audioSrc) {
  if (!audioContext) {
    audioContext = new (window.AudioContext || window.webkitAudioContext)();
  }

  const audio = new Audio(audioSrc);

  if (!analyser) {
    analyser = audioContext.createAnalyser();
    analyser.fftSize = 256;
  }

  const source = audioContext.createMediaElementSource(audio);
  source.connect(analyser);
  analyser.connect(audioContext.destination);

  audio.play();

  audio.onended = () => {
    updateStatus("IDLE");
    app.ticker.remove(lipSyncTicker);
    if (lainModel)
      lainModel.internalModel.coreModel.setParamFloat("PARAM_MOUTH_OPEN_Y", 0);
  };
  app.ticker.add(lipSyncTicker);

  window.currentAudio = audio;
}

const dataArray = new Uint8Array(128);
const lipSyncTicker = () => {
  if (!window.currentAudio || window.currentAudio.paused) return;

  analyser.getByteFrequencyData(dataArray);

  let sum = 0;
  for (let i = 0; i < dataArray.length; i++) {
    sum += dataArray[i];
  }
  const average = sum / dataArray.length;

  const mouthValue = Math.min(average * 0.03, 1.0);

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
    // backend expects the plain string "start_interaction"
    ws.send("start_interaction");

    if (audioContext && audioContext.state === "suspended") {
      audioContext.resume();
    }
  }
});

// --- Script loader with local fallback then CDN ---
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
      ["/static/js/vendor/pixi.min.js", "https://cdn.jsdelivr.net/npm/pixi.js@7.x/dist/pixi.min.js"],
      "PixiJS",
    );
    const live2dOk = await loadWithFallbacks(
      ["/static/js/vendor/live2d.min.js", "https://cdn.jsdelivr.net/gh/dizhenry/Live2D_v2/live2d.min.js"],
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
    initWebSocket(); // websocket can still work without Live2D
  }
}

window.addEventListener("load", bootstrap);
