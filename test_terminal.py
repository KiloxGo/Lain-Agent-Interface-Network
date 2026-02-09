import pyaudio
import wave
import os
from core.llm import LainMind
from core.audio import AudioSensor

CHUNK = 1024
FORMAT = pyaudio.paInt16
CHANNELS = 1
RATE = 16000
RECORD_SECONDS = 5
WAVE_OUTPUT_FILENAME = "test_input.wav"

def record_audio():
    p = pyaudio.PyAudio()
    stream = p.open(format=FORMAT,channels=CHANNELS,rate=RATE,input=True,frames_per_buffer=CHUNK)
    print("Recording...")
    frames = []
    for _ in range(0, int(RATE / CHUNK * RECORD_SECONDS)):
        data = stream.read(CHUNK)
        frames.append(data)
    print("Finished recording.")
    stream.stop_stream()
    stream.close()
    p.terminate()
    wf = wave.open(WAVE_OUTPUT_FILENAME, 'wb')
    wf.setnchannels(CHANNELS)
    wf.setsampwidth(p.get_sample_size(FORMAT))
    wf.setframerate(RATE)
    wf.writeframes(b''.join(frames))
    wf.close()

def play_audio(file):
    if not file: return
    print("Lain Speaking...")

    cmd = f"afplay {file}" if os.sys.platform == 'darwin' else f"mpg123 {file}"
    os.system(cmd)


def main():
    print("=== L.A.I.N. TERMINAL TEST ===")
    try:
        lain = LainMind()
        sensor = AudioSensor()
        print("Say something to Lain...")
    except Exception as e:
        print(f"Initialization Error: {e}")
        return
    
    while True:
        input("Press Enter to record (or Ctrl+C to exit)...")

        record_audio()

        print(f"Processing audio...")
        text = sensor.listen(WAVE_OUTPUT_FILENAME)
        print(f"You said: {text}")

        if not text: continue

        reply = lain.think(text)
        print(f"Lain replied: {reply}")

        audio_file = sensor.speak(reply)
        play_audio(audio_file)

if __name__ == "__main__":
    main()

