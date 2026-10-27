"""Records what misaki (Kokoro's own Chinese front end) says for the Mandarin corpus.

    misaki-venv/Scripts/python.exe -I crates/tts/tests/golden/misaki_reference.py

Writes mandarin-corpus.json (the texts) and mandarin-misaki.json (text -> phonemes) next to this
script. The Rust test `mandarin_matches_misaki` reads both, so it needs no Python. The corpus is
hand-written sentences plus seeded random sentences built from pypinyin's phrase list, which
stresses word segmentation and polyphones far beyond what a person would write.
"""

import json
import random
import re
from pathlib import Path

from misaki.zh import ZHG2P
from pypinyin.phrases_dict import phrases_dict

HERE = Path(__file__).resolve().parent

HAND = [
    "老灯塔守每天晚上都要爬上楼梯。",
    "你好，世界！",
    "今天天气很好，我们一起去公园散步吧。",
    "她说：“我明天不来了。”",
    "这本书一共有三百二十页，价格是四十九点八元。",
    "2024年5月12日，我们在北京见面。",
    "他的身高是1.75米，体重70公斤。",
    "百分之五十的人同意这个计划。",
    "今天的气温是25℃，明天可能会下雨。",
    "三分之二的学生通过了考试。",
    "银行里的钱不够，他只好去行政部门问问。",
    "我喜欢长城，它很长，也很长久。",
    "他还没有还我的书。",
    "这个重要的会议将在重庆举行。",
    "孩子们在操场上快乐地玩耍。",
    "“你为什么哭？”妈妈问。",
    "没有人知道他为什么离开了这座城市。",
    "海水慢慢退去，露出了满是泥土的滩涂。",
    "一二三四五六七八九十",
    "我们的故事从一个雨夜开始……",
    "请问，去火车站怎么走？",
    "春天来了；花开了；鸟儿在歌唱。",
    "这是第3章，共12节。",
    "《红楼梦》是中国古典文学的巅峰之作。",
    "他说了声“好”，然后转身离开。",
    "哪儿有卖水果的？我想买点苹果和香蕉。",
    "不，不要这样！",
    "东西放在桌子上了，你看看是不是你的东西。",
    "她笑着说：“谢谢你！”",
    "一个人的旅行并不孤单。",
    "银杏叶落满了整条街道，秋天真的来了。",
    "他花了1000000元买了一辆车。",
    "【重要】请在周五之前提交报告。",
    "（这是一个测试。）",
    "哈哈，你真逗！",
    "嗯，我知道了。",
    "是的，但是我不同意。",
    "我们应该爱护环境，保护地球。",
    "他在图书馆里读了一整天的书，直到天黑才回家。",
    "老燈塔守每天晚上都要爬上樓梯，一邊走一邊數著每一級台階。",
    "銀行裡的錢不夠，他只好去問問。",
    "這是一本關於海洋與風的書。",
    "我今天买了一部iPhone 15，花了5999元。",
    "他在Google工作，每天用Python写代码。",
]


def sentences(count, seed):
    rng = random.Random(seed)
    words = [w for w in phrases_dict if re.fullmatch("[一-鿿]{2,4}", w)]
    stops = ["，", "。", "！", "？", "；"]
    out = []
    for _ in range(count):
        parts = []
        for _ in range(rng.randint(3, 8)):
            parts.append("".join(rng.choice(words) for _ in range(rng.randint(1, 3))))
            parts.append(rng.choice(stops))
        out.append("".join(parts))
    return out


texts = HAND + sentences(400, 20240601)
g2p = ZHG2P()
expected = {text: g2p(text)[0] for text in texts}

with open(HERE / "mandarin-corpus.json", "w", encoding="utf8", newline="\n") as out:
    json.dump(texts, out, ensure_ascii=False, indent=1)
    out.write("\n")
with open(HERE / "mandarin-misaki.json", "w", encoding="utf8", newline="\n") as out:
    json.dump(expected, out, ensure_ascii=False, indent=1)
    out.write("\n")
print(len(texts), "texts")
