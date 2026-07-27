# 一纸手记 Landing Page

挂在 GitHub Pages 上的产品落地页。`/site` 目录是静态站本体。

## 本地预览

```bash
cd site && python3 -m http.server 8000
# 浏览器打开 http://localhost:8000
```

## 部署

push 到 `main` 分支后，GitHub Actions 会自动构建并部署到 GitHub Pages：
https://young920.github.io/voice-todo-float/

## 文件结构

```
site/
├── index.html          # 主页面
├── styles.css          # 墨夜缂丝风格
├── main.js             # tab 切换逻辑
├── assets/             # 产品截图
│   ├── todo.png
│   └── jinnang.png
└── README.md
```