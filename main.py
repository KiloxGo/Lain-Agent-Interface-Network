import base64
import asyncio
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse
import uvicorn

from core.llm import LainMind
from core.audio import AudioSensor
import pyaudio
import wave

app = FastAPI()

app.mount("/static", StaticFiles(directory="static"), name="static")

print("=== L.A.I.N. WEB SOCKET SERVER ===")
try:
    lain = LainMind()
    sensor = AudioSensor()
    print("LainMind and AudioSensor initialized successfully.")
except Exception as e:
    print(f"Initialization Error: {e}")

@app.get("/")
async def get():
    return FileResponse("static/index.html")

def sever_record_audio(filename="temp_input.wav", record_seconds=5):
    # Audio capture settings; commas removed so these stay integers, not tuples
    CHUNK = 1024
    FORMAT = pyaudio.paInt16
    CHANNELS = 1
    RATE = 16000

    p = pyaudio.PyAudio()
    stream = p.open(
        format=FORMAT,
        channels=CHANNELS,
        rate=RATE,
        input=True,
        frames_per_buffer=CHUNK,
    )
    
    print("Recording...")
    frames = []
    for _ in range(0, int(RATE / CHUNK * record_seconds)):
        data = stream.read(CHUNK)
        frames.append(data)
    print("Finished recording.")

    stream.stop_stream()
    stream.close()
    p.terminate()

    wf = wave.open(filename, 'wb')
    wf.setnchannels(CHANNELS)
    wf.setsampwidth(p.get_sample_size(FORMAT))
    wf.setframerate(RATE)
    wf.writeframes(b''.join(frames))
    wf.close()

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    print("Client Connected")

    # If initialization failed earlier, fail fast with a clear message instead of crashing
    if 'sensor' not in globals() or sensor is None or 'lain' not in globals() or lain is None:
        await websocket.send_json({"type": "status", "content": "server not ready"})
        await websocket.close()
        return

    try:
        while True:
            data = await websocket.receive_text()

            if data == "start_interaction":
                await websocket.send_json({"type": "status", "content": "listening"})
                sever_record_audio()
                await websocket.send_json({"type": "status", "content": "processing"})
                user_text = sensor.listen("temp_input.wav")
                print(f"User: {user_text}")

                await websocket.send_json({"type": "caption", "content": f"You:{user_text}"})
                
                if user_text:
                    reply = lain.think(user_text)
                    print(f"Lain: {reply}")
                    await websocket.send_json({"type": "caption", "content": f"Lain:{reply}"})

                    audio_file = sensor.speak(reply)

                    if audio_file:
                        with open(audio_file, "rb") as f:
                            audio_data = base64.b64encode(f.read()).decode('utf-8')
                        
                        await websocket.send_json({"type": "audio", "content": audio_data})
                else:
                    await websocket.send_json({"type": "status", "content": "idle"})


    except WebSocketDisconnect:
        print("Client Disconnected")
    except Exception as e:
        print(f"WebSocket Error: {e}")

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=5678)
