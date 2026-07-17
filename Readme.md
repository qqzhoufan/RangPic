# 咸鱼图床 · RangPic

> 自托管图床，**随机图只是其中一个功能**。Rust 编写，单二进制部署，主打功能齐全：多用户、文件夹、上传托管、随机图 API、相册与分享。

咸鱼图床（RangPic）最初是一个随机图片服务，现在已扩展为一座完整的自托管图床：多用户上传托管、WebP 缩略图、SHA-256 去重、**文件夹（相册）**、分享链接、可见性与定时过期、API Token、管理后台。其中**随机图**支持「某个公开文件夹」「某个用户的全部公开图片」「全站公开」三种范围，可作为博客背景图、头像、签名档等随机素材源。

前端为纯手工编写的设计系统（暖色调中性色 + 翡翠绿点缀，暗色画廊优先，无 AI 模板感），后端资源经 `rust-embed` 编译进单个二进制，**一个可执行文件 + 一个 PostgreSQL** 即可上线。

## 功能一览

| 模块 | 能力 |
| --- | --- |
| **上传托管** | 拖拽 / 点击 / **Ctrl+V 粘贴** / **URL 导入**，带真实上传进度；上传时可选择归属**文件夹** |
| **图片处理** | 自动生成 WebP 缩略图、**SHA-256 去重**、按用户**存储配额**限制、格式校验（JPG/PNG/GIF/WebP/BMP/ICO） |
| **可见性** | `公开` / `不公开`（凭链接可访问）/ `私有`（仅本人）；支持**定时过期** |
| **文件夹 / 相册** | 归档成集、设置可见性、封面，便于整理与分享 |
| **分享** | 分享页 `/s/{id}` 一键复制 直链 / Markdown / HTML / BBCode |
| **随机图 API** | `/random-image`、`/api/random-image`，支持标签、**文件夹**、**用户**三种范围过滤 |
| **账号体系** | 多用户注册登录（argon2id 密码哈希 + 数据库会话） |
| **API Token** | 为 PicGo、脚本、客户端签发令牌，走 `POST /api/upload` |
| **管理后台** | 站点统计、用户管理（角色 / 配额 MB / 封禁）、内容审核、**站点设置（含项目域名、默认配额）** |

## 技术栈

- **后端**：Rust · [Axum](https://crates.io/crates/axum) · Tokio · [SQLx](https://crates.io/crates/sqlx)（PostgreSQL，运行时校验，无编译期数据库依赖）
- **存储**：本地文件系统（`Storage` trait，预留 S3 等后端扩展）
- **认证**：argon2id + 服务端会话 + Bearer API Token
- **图像**：`image` crate（缩略图 / 格式识别）、`sha2`（去重哈希）
- **前端**：原生 HTML / CSS / JS，`rust-embed` 编译进二进制，无构建步骤

## 快速开始

### Docker（推荐）

```bash
git clone https://github.com/qqzhoufan/RangPic.git
cd RangPic
# 可先修改 docker-compose.yaml 里的 ADMIN_USERNAME / ADMIN_PASSWORD
docker compose up --build -d
```

打开 `http://localhost:17777`，用 compose 里设置的管理员账号登录（默认 `admin` / `changeme`，**请尽快在后台修改**）。

### 手动（Cargo）

需要 Rust **1.88+**（依赖的 `image`/`time` 等 MSRV 为 1.88，`icu_*` 为 1.86）与 PostgreSQL。

```bash
# 1. 建库
createdb rangpic

# 2. 配置环境变量（可复制 .env.example -> .env）
export DATABASE_URL="postgres://$USER@localhost:5432/rangpic"
export ADMIN_USERNAME=admin ADMIN_PASSWORD=adminpass

# 3. 运行（首次会自动建表 + 创建管理员 + 导入随机图种子）
cargo run --release
```

> 表结构在启动时由 `sqlx::migrate!` 自动创建（`migrations/`）。若 `images` 表为空，会从 `data/image_urls.txt`（每行 `url,tag1,tag2`）导入随机图池。

## 配置

### 环境变量

| 变量 | 默认 | 说明 |
| --- | --- | --- |
| `DATABASE_URL` | — | **必填**。PostgreSQL 连接串 |
| `ADMIN_USERNAME` / `ADMIN_PASSWORD` | `admin` / `adminpass` | **必填**。仅用于**首次启动**创建管理员账号；之后可在后台改密码 |
| `BIND_ADDR` | `0.0.0.0:17777` | 监听地址 |
| `SITE_URL` | 空 | 站点公网地址（仅作初始值；**后台可配置，优先级更高**），用于生成绝对分享/嵌入链接 |
| `STORAGE_DIR` | `storage` | 上传文件存储目录 |
| `MAX_UPLOAD_BYTES` | `26214400`（25 MiB） | 单文件大小上限 |
| `QUOTA_DEFAULT_BYTES` | `104857600`（100 MiB） | 单用户存储配额初始值（后台可配置；`0` 表示不限） |
| `REGISTRATION_OPEN` | `true` | 是否开放公开注册 |
| `SESSION_TTL_HOURS` | `336`（14 天） | 会话有效期 |
| `RUST_LOG` | `info` | 日志级别 |

### 运行时设置（后台 → 站点设置，无需改代码/重启域名相关）

- **项目域名**：生成绝对分享/嵌入链接用；留空则用相对链接。
- **新用户默认配额（MB）**：`0` 表示不限；管理员账号默认不限。
- 站点名称、站点描述、首页文案、是否开放注册。

## API 速查

### 随机图（公开，无需登录）

```
GET /random-image                         -> 图片字节（可作 <img src>），全站公开随机
GET /random-image?tags=mobile             -> 按标签过滤（大小写不敏感、子串匹配）
GET /random-image?album=<公开文件夹 id>    -> 仅从某个公开文件夹随机（私有文件夹返回 404）
GET /random-image?album=alice/travel      -> 同上，用「用户名/slug」指定
GET /random-image?user=alice              -> 仅从 alice 的全部公开图片随机
GET /api/random-image?...                 -> 同上参数，返回 JSON 元数据（含直链/嵌入代码）
GET /api/tags                             -> 随机图池中所有标签
GET /api/explore                          -> 最新公开图片
```

直接当 `<img src>` 用：

```html
<img src="https://你的域名/random-image?album=alice/travel" alt="随机图" />
```

### 上传（需登录或 API Token）

```bash
# 方式一：账号 Cookie
curl -c c.txt -X POST http://localhost:17777/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"secret"}'

curl -b c.txt -X POST http://localhost:17777/api/upload \
  -F file=@photo.jpg -F privacy=public -F tags=sky,blue -F album_id=<文件夹id>

# 方式二：API Token（PicGo / 脚本）
curl -X POST http://localhost:17777/api/upload \
  -H "Authorization: Bearer rpic_xxxxxxxx" \
  -F file=@photo.jpg
```

| 字段 | 说明 |
| --- | --- |
| `file` | 图片文件（JPG/PNG/GIF/WebP/BMP/ICO） |
| `privacy` | `public` / `unlisted` / `private` |
| `tags` | 逗号分隔的标签 |
| `album_id` | 可选，归属的文件夹（相册）ID |
| `expires_hours` | 可选，过期小时数（到期后图片不可访问） |

返回 `results[].image.links` 内含 `direct` / `markdown` / `html` / `bbcode` / `share` 等可直接复制的链接。

### 访问图片

```
GET /i/{id}   原图（按可见性鉴权）
GET /t/{id}   WebP 缩略图
GET /s/{id}   分享页（含元数据 + 嵌入代码）
```

## 项目结构

```
src/
  main.rs            启动、迁移、引导管理员、种子导入
  config.rs          环境变量配置
  state.rs           AppState（DB 池 / 存储 / HTTP 客户端 / 域名缓存）
  error.rs           统一错误类型 -> HTTP 响应
  auth.rs            密码哈希、会话、API Token、请求提取器
  storage.rs         Storage trait + 本地文件实现
  imaging.rs         格式识别、缩略图、SHA-256
  db.rs              SQLx 模型与列常量
  routes/            public / auth / upload / images / albums / admin / pages
migrations/          PostgreSQL 迁移（0001 schema · 0002 设置 · 0003 品牌）
static/              前端（CSS / JS / HTML 页面，编译时嵌入二进制）
data/image_urls.txt  随机图池种子数据
```

## 从旧版（Go）迁移

本项目由原 Go 版随机图服务用 Rust 重写而来，核心能力完整保留并增强：

- `data/image_urls.txt` 种子格式不变（每行 `url,tag1,tag2`），可直接复用。
- 随机图接口路径与参数（`tags`）保持兼容（同时接受 `tag`）。
- 原「本地素材库」升级为带缩略图、配额、去重、文件夹的正式上传系统。

> 仓库内仍保留了旧版 Go 源码（`cmd/`、`go.mod`、`go.sum`、`web/`）作为历史参考，它们**不参与**当前构建。如需纯 Rust 的干净仓库，可安全删除这些目录。

## License

MIT
