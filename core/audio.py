import os
from typing import Optional

import requests
from dotenv import load_dotenv

load_dotenv()

ASR_URL = "https://api.siliconflow.cn/v1/audio/transcriptions"
TTS_URL = "https://api.siliconflow.cn/v1/audio/speech"
DEFAULT_TTS_MODEL = "FunAudioLLM/CosyVoice2-0.5B"
DEFAULT_VOICE = "charles"


class AudioSensor:
    def __init__(self):
        self.api_key = os.getenv("SILICONFLOW_API_KEY")
        if not self.api_key:
            raise ValueError("SILICONFLOW_API_KEY is not set")

        self.session = requests.Session()
        self.session.headers.update({"Authorization": f"Bearer {self.api_key}"})

        self.tts_model = os.getenv("SILICONFLOW_TTS_MODEL", DEFAULT_TTS_MODEL)
        self.tts_voice = self._normalize_voice(os.getenv("SILICONFLOW_TTS_VOICE"))

    def _normalize_voice(self, voice: Optional[str]) -> str:
        voice = (voice or "").strip()
        if not voice:
            voice = DEFAULT_VOICE
        return voice if ":" in voice else f"{self.tts_model}:{voice}"

    def listen(self, audio_file_path: str) -> str:
        try:
            with open(audio_file_path, "rb") as f:
                files = {"file": f, "model": (None, "FunAudioLLM/SenseVoiceSmall")}
                response = self.session.post(ASR_URL, files=files, timeout=30)
                response.raise_for_status()
                return response.json().get("text", "")
        except Exception as e:
            print(f"ASR Error: {e}")
            return ""

    def speak(self, text: str, output_file: str = "reply.mp3") -> Optional[str]:
        if not text:
            return None

        payload = {
            "model": self.tts_model,
            "voice": self.tts_voice,
            "input": text,
            "response_format": "mp3",
            "sample_rate": 32000,
            "speed": 1.1,
        }
        try:
            response = self.session.post(TTS_URL, json=payload, timeout=60)
            if response.status_code == 200:
                with open(output_file, "wb") as f:
                    f.write(response.content)
                return output_file

            error_msg = response.text
            try:
                error_msg = response.json().get("message", error_msg)
            except Exception:
                pass
            print(f"TTS Error: {response.status_code} - {error_msg}")
            return None
        except Exception as e:
            print(f"TTS Error: {e}")
            return None
