# 咸鱼图床 · RangPic

> 自托管图床，**随机图只是其中一个功能**。用 Rust 编写，主打功能齐全：多用户、文件夹、上传托管、随机图 API、相册与分享。

咸鱼图床（RangPic）从一个随机图片服务起步，扩展成了一座完整的自托管图床：多用户、上传托管、WebP 缩略图、SHA-256 去重、**文件夹（相册）**、分享链接、过期与隐私、API Token、管理后台。其中随机图支持「**某个公开文件夹**」「**某个用户的全部公开图片**」「**全站公开**」三种范围。前端为纯手工编写的设计系统（暖色调中性色 + 翡翠绿点缀，暗色画廊优先），无构建步骤、单二进制部署。

## 功能一览

**图床能力（主打）**
- 上传时可选择归属的**文件夹（相册）**；支持拖拽 / 点击 / **Ctrl+V 粘贴** / URL 导入。
- 自动生成 WebP 缩略图、SHA-256 去重、按用户配额限制、可见性与定时过期。
- 文件夹整理、分享页（`/s/{id}`）一键复制直链 / Markdown / HTML / BBCode。

**随机图（众多功能之一）**
- `GET /random-image` 直接返回图片字节，可作为 `<img src>`。
- `GET /api/random-image` 返回 JSON 元数据（含直链 / Markdown / HTML / BBCode 嵌入代码）。
- 标签过滤：`?tags=desktop,nature`，大小写不敏感、子串匹配。
- **范围过滤**：`?album=<公开文件夹 id 或 用户名/slug>` 从某个公开文件夹随机；`?user=<用户名>` 从该用户的全部公开图片随机；不传则从全站公开图片随机。

**图床能力**
- 拖拽 / 点击 / **Ctrl+V 粘贴** / URL 导入 多种上传方式，带真实上传进度。
- 自动生成 WebP 缩略图；**SHA-256 去重**；按用户**存储配额**限制。
- 图片可见性：`公开` / `不公开`（凭链接可访问）/ `私有`（仅本人）；支持**定时过期**。
- **相册**整理；**分享页**（`/s/{id}`）含元数据与一键复制嵌入代码。
- **API Token**：为 PicGo、脚本、客户端签发令牌，走 `POST /api/upload`。
- **管理后台**：站点统计、用户管理（角色 / 配额 / 封禁）、内容审核、站点设置。

## 技术栈

- 后端：Rust + [Axum](https://crates.io/crates/axum) + Tokio + [SQLx](https://crates.io/crates/sqlx)（PostgreSQL）
- 存储：本地文件系统（`Storage` trait，可扩展 S3 等后端）
- 认证：argon2id 密码哈希 + 数据库会话 + Bearer API Token
- 前端：原生 HTML/CSS/JS，**前端资源经 rust-embed 编译进二进制**，单文件部署

## 快速开始

### Docker（推荐）

```bash
git clone https://github.com/qqzhoufan/RangPic.git
cd RangPic
# 修改 docker-compose.yaml 里的 ADMIN_PASSWORD 等
docker compose up --build -d
```

打开 `http://localhost:17777`，用 compose 里设置的管理员账号登录（默认 `admin` / `changeme`，**请尽快修改**）。

### 手动（Cargo）

需要 Rust **1.88+**（依赖的 `image`/`time` 等最低 MSRV 为 1.88，`icu_*` 为 1.86；Docker 构建会自动使用合适的版本）与 PostgreSQL。

```bash
# 1. 建库
createdb rangpic

# 2. 配置环境变量（可复制 .env.example -> .env）
export DATABASE_URL="postgres://$USER@localhost:5432/rangpic"
export ADMIN_USERNAME=admin ADMIN_PASSWORD=adminpass

# 3. 运行（首次会自动建表并创建管理员账号）
cargo run --release
```

> 数据库表会在启动时通过 `sqlx::migrate!` 自动创建。若 `images` 表为空，会从 `data/image_urls.txt`（每行 `url,tag1,tag2`）导入随机图池——兼容原 Go 版本的数据格式。

## 配置项

| 变量 | 默认 | 说明 |
| --- | --- | --- |
| `DATABASE_URL` | — | **必填**。PostgreSQL 连接串 |
| `ADMIN_USERNAME` / `ADMIN_PASSWORD` | `admin` / `adminpass` | **必填**。仅用于**首次启动**创建管理员账号 |
| `BIND_ADDR` | `0.0.0.0:17777` | 监听地址 |
| `SITE_URL` | 空 | 站点公网地址，用于生成绝对分享 / 嵌入链接 |
| `STORAGE_DIR` | `storage` | 上传文件存储目录 |
| `MAX_UPLOAD_BYTES` | `26214400`（25 MiB） | 单文件大小上限 |
| `QUOTA_DEFAULT_BYTES` | `5368709120`（5 GiB） | 单用户存储配额 |
| `REGISTRATION_OPEN` | `true` | 是否开放公开注册 |
| `SESSION_TTL_HOURS` | `336`（14 天） | 会话有效期 |
| `RUST_LOG` | `info` | 日志级别 |

## API 速查

### 随机图（公开）
```
GET /random-image                         -> 图片字节（可作 <img src>），全站公开随机
GET /random-image?tags=mobile             -> 按标签过滤
GET /random-image?album=<公开文件夹 id>    -> 仅从某个公开文件夹随机
GET /random-image?album=alice/travel      -> 同上，用 用户名/slug 指定
GET /random-image?user=alice              -> 仅从 alice 的全部公开图片随机
GET /api/random-image?...                 -> 同参数，返回 JSON 元数据
GET /api/tags                             -> 随机图池中所有标签
GET /api/explore                          -> 最新公开图片
```

### 上传（需登录或 Token）
```bash
# 用账号 Cookie
curl -c c.txt -X POST http://localhost:17799/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"secret"}'

curl -b c.txt -X POST http://localhost:17799/api/upload \
  -F file=@photo.jpg -F privacy=public -F tags=sky,blue

# 或用 API Token（PicGo / 脚本）
curl -X POST http://localhost:17799/api/upload \
  -H "Authorization: Bearer rpic_xxxxxxxx" \
  -F file=@photo.jpg
```

| 字段 | 说明 |
| --- | --- |
| `file` | 图片文件（JPG/PNG/GIF/WebP/BMP/ICO） |
| `privacy` | `public` / `unlisted` / `private` |
| `tags` | 逗号分隔 |
| `expires_hours` | 可选，过期小时数 |

返回 `results[].image.links` 内含 `direct` / `markdown` / `html` / `bbcode` / `share` 等可直接复制的链接。

### 访问图片
```
GET /i/{id}   原图（按可见性鉴权）
GET /t/{id}   WebP 缩略图
GET /s/{id}   分享页
```

## 项目结构

```
src/
  main.rs            启动、迁移、引导管理员、种子导入
  config.rs          环境变量配置
  state.rs           AppState（DB 池 / 存储 / HTTP 客户端）
  error.rs           统一错误类型 -> HTTP 响应
  auth.rs            密码哈希、会话、API Token、请求提取器
  storage.rs         Storage trait + 本地文件实现
  imaging.rs         格式识别、缩略图、SHA-256
  db.rs              SQLx 模型与列常量
  routes/            public / auth / upload / images / albums / admin / pages
migrations/          PostgreSQL 迁移
static/              前端（CSS / JS / HTML 页面，编译时嵌入）
data/image_urls.txt  随机图池种子数据
```

## 从旧版（Go）迁移

原 Go 版本的核心能力（随机图、标签过滤、`image_urls.txt` 种子）已完整移植并增强：
- `data/image_urls.txt` 格式不变，可直接复用。
- 随机图接口路径与参数（`tags`）保持兼容（同时接受 `tag`）。
- 原“本地素材库”升级为带缩略图、配额、去重的正式上传系统。

## License

MIT
