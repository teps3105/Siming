# away 背過身（不甩袖）

40 格、不循環。事件：`turned` = 20（翻面完成）。

## 關鍵格
0–6 近手抬到胸前預備；6–12 甩袖橫掃：近手向後上、遠手向前、風量 0.2、衣擺橫飄，整個人翻面（flip 1 → −1，12 格是側薄的一線）；20 turned：前傾 26°、頭深低、遠手掩面、近手抱在胸前，肩線下沉；20–30 停住；36 放鬆；40 站定（背對原方向）。

## 採用的原則與來源
- 剪影測試：把姿勢塗黑、拉遠看，不看字卡也要看得出在做什麼；原文「if I have to explain my pose, it's not speaking loudly enough on its own」；一條動作線只說一件事；關鍵格必須在黑色剪影下讀得出，格與格之間可以鬆，重要姿勢停久、過渡加快。來源：https://anim.works/silhouette-in-animation/
- 預備與跟隨、誇大、清楚的舞台安排（Disney 十二原則）：大動作前先有反向的預備，動作到位後衣袖衣擺晚一拍才停；誇大到遠處仍讀得出。來源：https://mf.media.mit.edu/courses/2006/mas845/readings/files/DisneyAll.pdf （只取得搜尋摘要，全文未能讀取）
- 戲曲水袖與皮影的身段：水袖是接在袖口的長袖，用來放大情緒與動作，靠甩、抖、拋；皮影人是側面剪影，靠關節的大幅擺動表現。這兩項來源只取得搜尋摘要，全文未能讀取：https://www.instagram.com/popular/peking-opera-water-sleeves/ ；https://spectrum.library.concordia.ca/985304/1/Rollins_PhD_S2019.pdf
- 實作：以 2D 關節直接寫關鍵姿勢（tools/figure/model/poses2d.py），不經影片模型；格數與事件如下。
