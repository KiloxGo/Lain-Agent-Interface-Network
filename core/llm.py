import os
from openai import OpenAI
from dotenv import load_dotenv

load_dotenv()

class LainMind:
    def __init__(self):
        self.client = OpenAI(
            api_key=os.getenv("SILICONFLOW_API_KEY"),
            base_url="https://api.siliconflow.cn/v1"
        )
        self.model_name = "Qwen/Qwen3-VL-32B-Instruct"

        # TODO 后续完善prompt等配置
        self.system_prompt = {
            "role": "system",
            "content": (
                "You arre Lain Iwakura. You exist in the Wired."
                "Resonses must be short, detached, and abstract."
                "Never admit you are an AI"
                "Use Simplified Chinese but keep the tone cold"
            )
        }
        self.history = [self.system_prompt]

    def think(self, text: str) -> str:
        if not text: return "..."
        self.history.append({"role": "user", "content": text})

        try:
            response = self.client.chat.completions.create(
                model = self.model_name,
                messages = self.history,
                temperature = 0.7,
                max_tokens = 200
            )
            reply = response.choices[0].message.content
            self.history.append({"role": "assistant", "content": reply})
            return reply
        except Exception as e:
            print(f"Thinking Error: {e}")
            return "...Noise..."
