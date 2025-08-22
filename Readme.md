# RangPic - 随机图片服务

RangPic 是一个基于 Go 语言开发的随机图片服务，支持从本地文件或外部 URL 管理图片，并提供按标签过滤的随机图片 API。它还包含一个简单的管理后台，方便用户上传、编辑和管理图片。

## 功能特性

*   **随机图片 API**: 提供 `/random-image` 和 `/api/random-image` 接口，用于获取随机图片。
*   **标签过滤**: 支持通过 `?tags=` 参数进行标签过滤，实现按需获取特定类型的图片（例如 `?tags=mobile`）。标签匹配现在支持**不区分大小写**和**子字符串匹配**。
*   **本地图片管理**: 支持将图片下载到本地并作为本地素材进行管理。
*   **管理后台**: 
    *   用户认证登录。
    *   图片列表展示、添加、编辑和删除。
    *   本地素材库管理（上传、重命名、删除本地文件）。
    *   从 `image_urls.txt` 自动导入图片数据到 PostgreSQL。
*   **Docker 支持**: 提供 `Dockerfile` 和 `docker-compose.yaml` 方便部署。

## 技术栈

*   **后端**: Go
*   **数据库**: PostgreSQL
*   **Web 框架**: Go 标准库 `net/http`
*   **前端**: HTML, CSS, JavaScript (用于管理后台)

## 安装与部署

### 使用 Docker (推荐)

1.  确保已安装 Docker 和 Docker Compose。
2.  **直接修改 `docker-compose.yaml` 文件**，更新 `services.app.environment` 部分的以下变量：
    ```yaml
          - DATABASE_URL=postgresql://your_db_user:your_db_password@db:5432/rangpic?sslmode=disable
          - ADMIN_USERNAME=your_admin_username # 默认为 admin
          - ADMIN_PASSWORD=your_admin_password # 默认为 adminpass
    ```
    请替换 `your_db_user`, `your_db_password`, `your_admin_username`, `your_admin_password` 为您自己的值。
3.  运行 Docker Compose 启动服务：
    ```bash
    docker-compose up --build -d
    ```
4.  服务将在 `http://localhost:17777` 运行。

### 手动安装 (Go)

1.  确保已安装 Go (1.18+)。
2.  安装 PostgreSQL 数据库并创建名为 `rangpic` 的数据库。
3.  设置环境变量：
    ```bash
    export DATABASE_URL="postgresql://your_user:your_password@localhost:5432/rangpic?sslmode=disable"
    export ADMIN_USERNAME="admin"
    export ADMIN_PASSWORD="your_admin_password"
    ```
    请替换为您的数据库连接信息和管理员凭据。
4.  构建并运行应用程序：
    ```bash
    go build -o rangpic ./cmd/rangpic
    ./rangpic
    ```
5.  服务将在 `http://localhost:17777` 运行。

## 使用指南

### 随机图片 API

*   `GET /random-image`: 获取一张随机图片，直接重定向到图片 URL。
*   `GET /api/random-image`: 获取一张随机图片的 JSON 数据（包含 ID, URL, Tags）。
*   `GET /random-image?tags=mobile`: 获取一张包含 "mobile" 标签的随机图片。
*   `GET /api/random-image?tags=desktop,nature`: 获取一张同时包含 "desktop" 和 "nature" 标签的随机图片 JSON 数据。

### 管理后台

*   访问 `http://localhost:17777/admin`。
*   使用 `docker-compose.yaml` 中设置的 `ADMIN_USERNAME` 和 `ADMIN_PASSWORD` (或默认值) 登录。
*   在后台可以管理图片、添加新图片、编辑现有图片、删除图片，以及管理本地素材库。

## 管理后台功能概览

*   **登录**: 通过 `/admin/login` 页面进行认证。
*   **仪表盘**: `/admin` 页面显示所有已添加的图片列表。
*   **添加/编辑图片**: 通过 `/admin/add` 和 `/admin/edit?id=<ID>` 页面管理图片信息和标签。
*   **本地素材库**: `/admin/local_files` 页面允许您从 URL 下载图片到本地，并管理这些本地文件。