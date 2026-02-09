import os
from pathlib import Path
from typing import Optional

import requests
from dotenv import load_dotenv

load_dotenv()

ASR_URL = "https://api.siliconflow.cn/v1/audio/transcriptions"
TTS_URL = "https://api.siliconflow.cn/v1/audio/speech"
DEFAULT_TTS_MODEL = "FunAudioLLM/CosyVoice2-0.5B"
DEFAULT_VOICE = "charles"
VOICE_UPLOAD_URL = "https://api.siliconflow.cn/v1/uploads/audio/voice"
DEFAULT_REF_TEXT = "私の名前は岩倉麗音です。皆さんと知り合えて嬉しいです。"
DEFAULT_REF_FILE = "static/lain_ref.wav"


class AudioSensor:
    def __init__(self):
        self.api_key = os.getenv("SILICONFLOW_API_KEY")
        if not self.api_key:
            raise ValueError("SILICONFLOW_API_KEY is not set")

        self.session = requests.Session()
        self.session.headers.update({"Authorization": f"Bearer {self.api_key}"})

        self.tts_model = os.getenv("SILICONFLOW_TTS_MODEL", DEFAULT_TTS_MODEL)
        self.voice_cache_path = Path(os.getenv("SILICONFLOW_TTS_VOICE_CACHE", "core/.lain_voice_uri"))

        ref_voice = self._load_cached_voice_uri()
        if not ref_voice:
            ref_voice = self._upload_reference_voice()

        if ref_voice:
            self.tts_voice = self._normalize_voice(ref_voice)
            try:
                self.voice_cache_path.write_text(self.tts_voice)
            except Exception as e:
                print(f"Voice cache write failed: {e}")
        else:
            env_voice = os.getenv("SILICONFLOW_TTS_VOICE")
            self.tts_voice = self._normalize_voice(env_voice)

    def _normalize_voice(self, voice: Optional[str]) -> str:
        voice = (voice or "").strip()
        if not voice:
            voice = DEFAULT_VOICE
        return voice if ":" in voice else f"{self.tts_model}:{voice}"

    def _load_cached_voice_uri(self) -> Optional[str]:
        try:
            if self.voice_cache_path.is_file():
                return self.voice_cache_path.read_text().strip()
        except Exception as e:
            print(f"Voice cache read failed: {e}")
        return None

    def _upload_reference_voice(self) -> Optional[str]:
        """
        Upload local reference audio to SiliconFlow to obtain a custom voice URI.
        Falls back silently if reference file is missing or upload fails.
        """
        ref_path = Path(os.getenv("SILICONFLOW_TTS_REF_FILE", DEFAULT_REF_FILE))
        if not ref_path.exists():
            print(f"Reference audio not found: {ref_path}")
            return None

        custom_name = os.getenv("SILICONFLOW_TTS_CUSTOM_NAME", "lain-ref")
        ref_text = os.getenv("SILICONFLOW_TTS_REF_TEXT", DEFAULT_REF_TEXT)

        files = {"file": open(ref_path, "rb")}
        data = {
            "model": self.tts_model,
            "customName": custom_name,
            "text": ref_text,
        }

        try:
            resp = self.session.post(VOICE_UPLOAD_URL, files=files, data=data, timeout=60)
            resp.raise_for_status()
            uri = resp.json().get("uri")
            if uri:
                print(f"Reference voice uploaded: {uri}")
                return uri
            print(f"Upload response missing uri: {resp.text}")
        except Exception as e:
            print(f"Voice upload failed: {e}")
        finally:
            try:
                files["file"].close()
            except Exception:
                pass
        return None

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
