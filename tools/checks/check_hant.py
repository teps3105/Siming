#!/usr/bin/env python3
"""檢查繁體中文：找出簡體字（本地作業規範、GDD 6.7 測試與工具）。

掃描範圍：
  - docs/**/*.md（全文）
  - native/sim/rust/crates/**/data/*.json、game/modules/locale/resources/*.json（\\uXXXX 先解碼後逐字掃描）
  - native/sim/rust/crates/**/*.rs 的字串常值與註解（識別字與程式碼本身不掃描）

對照表為「簡體 → 繁體」常用字（兩字一組，見 SIMP_TRAD_PAIRS）。只收錄簡繁字形不同、
且簡體字本身不是常用繁體字的字，以避免誤判；例如「里」「台」「后」「面」「干」「余」
「只」「才」「么」「几」「云」「占」「制」「松」「于」「郁」「咸」「吊」等同時為合法繁體字者不收錄。

用法：
    python3 tools/check_hant.py [--root <repo 根目錄>]

輸出每筆「檔案:行:欄: 簡→繁」，最後印摘要；有發現則以 1 結束，否則以 0 結束。
"""

import argparse
import re
import sys
from pathlib import Path

# 每個 token 為「簡體＋繁體」兩個字，以空白分隔。
SIMP_TRAD_PAIRS = """
这這 为為 设設 计計 场場 图圖 蚀蝕 洼窪 构構 们們 个個 说說 对對 时時 会會 经經 过過
还還 进進 边邊 动動 务務 区區 单單 当當 点點 电電 东東 丽麗 两兩 严嚴 丧喪 并併 临臨
丰豐 举舉 义義 乐樂 乔喬 习習 乡鄉 书書 买買 乱亂 争爭 亏虧 亚亞 产產 亩畝
亲親 亿億 仅僅 从從 仓倉 价價 众眾 优優 伙夥 伤傷 传傳 伦倫 伪偽 体體 佣傭 侠俠 侣侶
侦偵 侧側 俭儉 债債 倾傾 偿償 储儲 儿兒 党黨 兰蘭 关關 兴興 养養 兽獸 冲沖 决決 况況
冯馮 凉涼 减減 凤鳳 凭憑 击擊 凿鑿 刍芻 划劃 刘劉 则則 刚剛 创創 删刪 别別 剂劑 剧劇
劝勸 办辦 动動 励勵 劲勁 势勢 勋勳 匀勻 华華 协協 卖賣 卢盧 卤鹵 卫衛 厅廳 历歷 厉厲
压壓 厌厭 县縣 参參 双雙 变變 叙敘 叶葉 号號 叹嘆 吓嚇 吕呂 吗嗎 吨噸 听聽 启啟 吴吳
员員 呛嗆 呜嗚 咏詠 咙嚨 响響 哑啞 哟喲 唤喚 啬嗇 喷噴 嗫囁 团團 围圍 园園 圆圓 圣聖
圹壙 坏壞 块塊 坚堅 坛壇 坝壩 坟墳 垄壟 垒壘 垦墾 垫墊 埘塒 埙塤 堑塹 墙牆 壮壯 声聲
壳殼 壶壺 处處 备備 复復 夸誇 夺奪 奁奩 奂奐 奋奮 奖獎 妆妝 妇婦 妈媽 娄婁 孙孫 学學
宁寧 宝寶 实實 宠寵 审審 宪憲 宫宮 宽寬 宾賓 寻尋 尔爾 尝嘗 层層 屿嶼 岁歲 岂豈 岗崗
岭嶺 峡峽 帐帳 带帶 帧幀 幂冪 库庫 应應 庐廬 废廢 开開 异異 弃棄 张張 弥彌 弹彈 强強
归歸 录錄 彻徹 忆憶 忧憂 怀懷 态態 怜憐 总總 恳懇 恶惡 恼惱 悬懸 惊驚 惧懼 惨慘
惯慣 愤憤 战戰 戏戲 户戶 扑撲 执執 扩擴 扪捫 扫掃 扬揚 扰擾 抚撫 抛拋 抟摶 抠摳 护護
报報 担擔 拥擁 拦攔 拧擰 择擇 挟挾 挥揮 据據 掳擄 掷擲 揽攬 摆擺 摇搖 摈擯 携攜 数數
斋齋 旧舊 显顯 晒曬 晓曉 晖暉 暂暫 术術 机機 杀殺 杂雜 权權 条條 来來 杨楊 杰傑
柜櫃 柠檸 栅柵 标標 树樹 栉櫛 桥橋 桩樁 梦夢 检檢 楼樓 榄欖 欢歡 欤歟 歼殲 残殘 毕畢
毙斃 气氣 汇匯 汉漢 汤湯 沟溝 沥瀝 沦淪 泞濘 没沒 泪淚 泼潑 泽澤 洁潔 洒灑 浅淺 测測
浆漿 浇澆 浓濃 涂塗 涛濤 涝澇 湾灣 湿濕 滚滾 满滿 滤濾 滥濫 滨濱 灭滅 灯燈 灵靈 灾災
炉爐 炼煉 烁爍 烂爛 热熱 焕煥 爱愛 牵牽 犊犢 状狀 猎獵 猫貓 献獻 玛瑪 环環 现現 玺璽
独獨 狮獅 画畫 疗療 痒癢 瘫癱 皱皺 盏盞 监監 盖蓋 盗盜 盘盤 睁睜 矫矯 砖磚 矶磯 码碼
础礎 硕碩 确確 礼禮 离離 种種 积積 称稱 税稅 穷窮 窃竊 窝窩 竞競 笔筆 笋筍 筑築 筛篩
签簽 简簡 类類 粮糧 紧緊 纪紀 纬緯 纯純 纱紗 纲綱 纳納 纵縱 纸紙 线線 组組 细細 织織
终終 绅紳 维維 绊絆 绍紹 结結 绕繞 绘繪 给給 络絡 绝絕 统統 继繼 绩績 绪緒 续續 缆纜
缓緩 编編 缘緣 罗羅 罚罰 网網 罢罷 羁羈 聪聰 肃肅 肠腸 肤膚 肮骯 肾腎 胁脅 脉脈 脏臟
脑腦 腊臘 腻膩 舰艦 艳豔 艺藝 节節 芦蘆 苇葦 苍蒼 茧繭 荆荊 荐薦 药藥 莱萊 获獲
萝蘿 蒋蔣 蔼藹 蔷薔 虏虜 虚虛 虫蟲 蚁蟻 蚕蠶 蛮蠻 补補 衬襯 衅釁 视視 见見 观觀 规規
觅覓 览覽 许許 诉訴 证證 试試 诗詩 话話 该該 详詳 誉譽 贝貝 负負 贞貞 贡貢 财財 责責
败敗 货貨 质質 贩販 贪貪 贫貧 贮貯 贯貫 贱賤 贴貼 贵貴 贸貿 费費 资資 赋賦 赏賞 赐賜
赔賠 赖賴 赞贊 赠贈 赢贏 趋趨 踊踴 车車 轧軋 轨軌 轮輪 软軟 轰轟 轴軸 轻輕 载載 较較
辅輔 辆輛 辈輩 辉輝 辞辭 农農 迟遲 迹跡 逊遜 连連 运運 达達 迈邁 违違 遗遺 邮郵
邹鄒 邓鄧 郑鄭 酱醬 释釋 针針 钉釘 钓釣 钢鋼 钥鑰 钦欽 钩鉤 钮鈕 钱錢 钻鑽 铁鐵 铃鈴
铅鉛 铜銅 银銀 锁鎖 锅鍋 锈鏽 锋鋒 键鍵 锦錦 错錯 锤錘 镜鏡 镇鎮 长長 闪閃 闭閉 问問
闯闖 闲閒 间間 闹鬧 闻聞 阅閱 阔闊 队隊 阳陽 阵陣 阶階 际際 陆陸 陈陳 陕陝 险險
隐隱 随隨 隶隸 难難 雏雛 霁霽 静靜 韧韌 顶頂 项項 顺順 须須 顽頑 顾顧 顿頓 颁頒 颂頌
预預 领領 频頻 颓頹 题題 颜顏 额額 风風 飞飛 饥飢 饭飯 饮飲 饰飾 饱飽 驱驅 驳駁 驴驢
驶駛 驷駟 骑騎 验驗 骤驟 鱼魚 鲁魯 鲜鮮 鸟鳥 鸡雞 鸣鳴 鸭鴨 鸯鴦 鹅鵝 鹤鶴 鹰鷹 麦麥
黄黃 齐齊 齿齒 龙龍 龟龜 龄齡 讲講 讶訝 讽諷 论論 访訪 诀訣 询詢 语語 误誤 说說 请請
调調 谁誰 课課 谈談 谋謀 谎謊 谓謂 谱譜 谨謹 识識 译譯 议議 讯訊 仪儀 伟偉 侨僑 俩倆
劳勞 匮匱 吁籲 垩堊 奸姦 妩嫵 娇嬌 婴嬰 孪孿 寿壽 将將 尘塵 尴尷 巩鞏 帅帥 师師 庙廟
悭慳 惩懲 慑懾 戆戇 拟擬 挚摯 掴摑 撑撐 撵攆 撷擷 攒攢 敌敵 敛斂 斓斕 无無 旷曠 昼晝
晋晉 晕暈 暧曖 杆桿 枢樞 枣棗 栈棧 桠椏 桢楨 梼檮 椁槨 榇櫬 榉櫸 槛檻 槟檳 樯檣 橱櫥
橹櫓 殒殞 毁毀 毂轂 氢氫 汹洶 泷瀧 淀澱 渊淵 渗滲 渐漸 溃潰 滞滯 灏灝 炀煬 烧燒 焘燾
犹猶 狱獄 玑璣 珑瓏 琏璉 琐瑣 瑷璦 璎瓔 痪瘓 皑皚 皲皸 盐鹽 睑瞼 睐睞 矾礬 矿礦 硖硤
碍礙 祸禍 禀稟 稳穩 窍竅 窥窺 窦竇 窑窯 窖窖 筚篳 筹籌 篮籃 篱籬 粝糲 粤粵 糁糝 综綜
绦絛 绨綈 绮綺 绫綾 绸綢 绺綹 绻綣 绾綰 缀綴 缁緇 缅緬 缇緹 缉緝 缎緞 缔締 缗緡 骊驪
瓯甌 诸諸 谕諭 谬謬 谐諧 谍諜 谦謙 谧謐 谊誼 谑謔 谣謠 迁遷 选選 递遞 逻邏 遥遙
邻鄰 酝醞 酿釀 钵鉢 钜鉅 钞鈔 钟鐘 铠鎧 铲鏟 铺鋪 铭銘 铸鑄 铿鏗 销銷 锐銳 锡錫 锣鑼
锥錐 锭錠 镀鍍 镰鐮 镶鑲 阁閣 闸閘 闺閨 阀閥 隽雋 雾霧 霭靄 韩韓 韵韻 页頁 颇頗 颊頰
饪飪 饲飼 饴飴 饺餃 饼餅 馆館 馈饋 馋饞 驮馱 驯馴 驰馳 驻駐 驾駕 骂罵 骆駱 骗騙 骚騷
鬓鬢 魇魘 鲇鮎 鲍鮑 鲛鮫 鲞鯗 鲠鯁 鲨鯊 鲷鯛 鳄鱷 鳍鰭 鳖鱉 鳞鱗 鸠鳩 鸢鳶 鸦鴉 鸽鴿
鸾鸞 鹃鵑 鹊鵲 鹏鵬 鹜鶩 麸麩 麽麼 黉黌 黩黷 齑齏 诞誕 净淨 诡詭 诱誘 嘘噓 摄攝
滩灘 摊攤 陨隕 辽遼 沧滄 澜瀾 泻瀉 渔漁 浑渾 浊濁 温溫 饿餓 愿願 怅悵 懒懶 桨槳
栖棲 殁歿 烟煙 烦煩 烛燭 熏薰 猪豬 瑶瑤 疯瘋 瞩矚 瞒瞞 禅禪 籁籟 绣繡 绿綠 缩縮
翘翹 耸聳 聂聶 胧朧 脸臉 腾騰 舆輿 艰艱 苏蘇 莲蓮 萧蕭 蓝藍 薮藪 虑慮 蜡蠟 蝇蠅
袄襖 裤褲 讳諱 诚誠 诫誡 谅諒 谏諫 谭譚 讼訟 轩軒 轼軾 鉴鑒 雇僱 雳靂 霉黴 韬韜
颈頸 颗顆 颠顛 饶饒 骇駭 骡騾 髅髏 鲤鯉 鳅鰍 鳌鰲 鳗鰻 鹉鵡 恋戀 挂掛 缠纏 涩澀
纷紛 赶趕 讹訛 讷訥 讪訕 诈詐 诏詔 诅詛 迩邇 颍潁 绎繹 绑綁 蕴蘊 荣榮 萤螢
营營 莺鶯 蔺藺 蓟薊 胜勝 赡贍 砺礪 砾礫 碱鹼 秽穢 窜竄 笺箋 箫簫 篑簣 粪糞 购購
贾賈 贺賀 赁賃 赌賭 贼賊 赃贓 账賬 讨討 训訓 让讓 爷爺 夹夾 竖豎 叠疊 哗嘩 嘱囑
辩辯 辫辮 辖轄 纩纊 写寫 门門 马馬 韦韋 觉覺 读讀 导導 认認 样樣 发發 医醫 虽雖
乌烏 尽盡 挤擠 挣掙 悦悅 荡蕩 蜗蝸 欧歐 虾蝦 沟溝 闷悶 陈陳 烧燒 盗盜 声聲 设設
党黨 礼禮 宽寬 将將 观觀 驿驛 饭飯 赞贊 鸡雞 钦欽 讯訊 亿億
业業 阴陰 冈岡 殓殮 颐頤 赵趙 谢謝 凄淒 惬愜 缴繳 伞傘 屉屜 驼駝 轿轎 腌醃 剑劍 颅顱 纤纖 烨燁 锄鋤 鹭鷺 发發 写寫 门門 马馬 韦韋 觉覺 读讀 导導 认認 样樣 医醫 虽雖 乌烏 尽盡 挤擠 挣掙 悦悅 荡蕩 蜗蝸 欧歐 虾蝦 闷悶 烧燒 盗盜 党黨
"""


# 刻意保留的簡體字（規格上需要），以「檔案相對路徑: [(起行, 迄行, 理由)]」登記。
# 新增項目前須確認確實是刻意的；行號變動時請同步更新。
ALLOW = {
    "docs/1-概述/1.7-命名候選.md": [
        (60, 60, "說明簡體版產品名（簡體「司业命缘」）"),
    ],
    "docs/9-發行/9.5-在地化.md": [
        (31, 39, "在地化對照表的簡體譯名欄"),
    ],
}


def _allowed(rel, line):
    for start, end, _reason in ALLOW.get(rel, []):
        if start <= line <= end:
            return True
    return False


def _load_table():
    table = {}
    bad = []
    for tok in SIMP_TRAD_PAIRS.split():
        if len(tok) != 2:
            bad.append(tok)
            continue
        simp, trad = tok[0], tok[1]
        if simp != trad:
            table.setdefault(simp, trad)
    if bad:
        raise SystemExit(f"對照表格式錯誤（應為兩字一組）：{bad}")
    return table


S2T = _load_table()

UESC = re.compile(r"\\u([0-9a-fA-F]{4})")
RUST_UESC = re.compile(r"\\u\{([0-9a-fA-F]{1,6})\}")


def scan_text(text, path, out):
    """逐行、逐字掃描，將簡體字加入 out。"""
    for ln, line in enumerate(text.splitlines(), start=1):
        for col, ch in enumerate(line, start=1):
            if ch in S2T:
                out.append((path, ln, col, ch, S2T[ch]))


def rust_literals_and_comments(text):
    """回傳 (行號, 文字) 片段：只含字串常值與註解內容，忽略程式碼本身。"""
    segs = []
    i, n, line = 0, len(text), 1
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            segs.append((line, text[i:j]))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            seg = text[i:j]
            for k, part in enumerate(seg.split("\n")):
                segs.append((line + k, part))
            line += seg.count("\n")
            i = j
        elif c == '"':
            j = i + 1
            buf = []
            start_line = line
            while j < n and text[j] != '"':
                if text[j] == "\\" and j + 1 < n:
                    buf.append(text[j:j + 2])
                    if text[j + 1] == "\n":
                        line += 1
                    j += 2
                    continue
                if text[j] == "\n":
                    line += 1
                buf.append(text[j])
                j += 1
            for k, part in enumerate("".join(buf).split("\n")):
                segs.append((start_line + k, part))
            i = j + 1
        else:
            i += 1
    return segs


def scan_rust(path, out):
    text = path.read_text(encoding="utf-8")
    for ln, seg in rust_literals_and_comments(text):
        seg = RUST_UESC.sub(lambda m: chr(int(m.group(1), 16)), seg)
        for col, ch in enumerate(seg, start=1):
            if ch in S2T:
                out.append((path, ln, col, ch, S2T[ch]))


def scan_json(path, out):
    text = path.read_text(encoding="utf-8")
    # \uXXXX 解碼後不含換行，行號不受影響。
    decoded = UESC.sub(lambda m: chr(int(m.group(1), 16)), text)
    scan_text(decoded, path, out)


def scan_md(path, out):
    scan_text(path.read_text(encoding="utf-8"), path, out)


def collect_targets(root):
    targets = []
    docs = root / "docs"
    if docs.is_dir():
        targets += [(p, scan_md) for p in sorted(docs.rglob("*.md"))]
    crates = root / "native" / "sim" / "rust" / "crates"
    if crates.is_dir():
        targets += [(p, scan_json) for p in sorted(crates.rglob("data/*.json"))]
        targets += [(p, scan_rust) for p in sorted(crates.rglob("*.rs"))]
    locale = root / "game" / "modules" / "locale" / "resources"
    if locale.is_dir():
        targets += [(p, scan_json) for p in sorted(locale.glob("*.json"))]
    return targets


def main(argv=None):
    ap = argparse.ArgumentParser(description="檢查簡體字（繁體中文規範）")
    ap.add_argument("--root", default=None,
                    help="repo 根目錄（預設為本腳本往上兩層）")
    args = ap.parse_args(argv)
    root = Path(args.root).resolve() if args.root else Path(__file__).resolve().parents[2]

    raw = []
    targets = collect_targets(root)
    for path, scan in targets:
        scan(path, raw)

    found, allowed = [], 0
    for path, ln, col, ch, sug in raw:
        rel = path.relative_to(root).as_posix()
        if _allowed(rel, ln):
            allowed += 1
            continue
        found.append((path, ln, col, ch, sug))
        print(f"{rel}:{ln}:{col}: {ch}→{sug}")

    files_with = len({f[0] for f in found})
    print(f"掃描檔案 {len(targets)} 個；對照表 {len(S2T)} 組；"
          f"簡體字 {len(found)} 處（{files_with} 個檔案）；豁免 {allowed} 處")
    if found:
        print("FAIL: 發現簡體字")
        return 1
    print("OK: 未發現簡體字")
    return 0


if __name__ == "__main__":
    sys.exit(main())
