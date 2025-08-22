package main

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"html/template"
	"io"
	"log"
	"math/rand"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v4"
	"github.com/jackc/pgx/v4/pgxpool"
)

// --- 数据结构 ---

type Image struct {
	ID   int      `json:"id"`
	URL  string   `json:"url"`
	Tags []string `json:"tags"`
}

type EditPageData struct {
	Image     Image
	IsDesktop bool
	IsMobile  bool
	OtherTags string
}

type LocalFile struct {
	Name    string
	ModTime time.Time
}

// PageData a struct to hold all the data for the templates
type PageData struct {
	Title        string
	UserLoggedIn bool
	Images       []Image
	Data         interface{}
	LocalFiles   []LocalFile
}

const localImagesPath = "/app/local_images"

var (
	dbpool        *pgxpool.Pool
	adminUsername string
	adminPassword string
	sessions      = make(map[string]bool)
	httpClient    = &http.Client{Timeout: 15 * time.Second}
	templates     *template.Template
)

// --- 主函数和初始化 ---

func main() {
	rand.Seed(time.Now().UnixNano())
	loadConfig()

	var err error
	dbpool, err = pgxpool.Connect(context.Background(), os.Getenv("DATABASE_URL"))
	if err != nil {
		log.Fatalf("无法连接到 PostgreSQL: %v", err)
	}
	defer dbpool.Close()

	if err := initDB(context.Background()); err != nil {
		log.Fatalf("数据库初始化失败: %v", err)
	}

	parseTemplates()
	setupRoutes()

	port := "17777"
	log.Printf("服务器启动在 http://localhost:%s", port)
	log.Fatal(http.ListenAndServe(":"+port, nil))
}

func loadConfig() {
	databaseUrl := os.Getenv("DATABASE_URL")
	if databaseUrl == "" {
		log.Fatal("DATABASE_URL 环境变量未设置")
	}
	adminUsername = os.Getenv("ADMIN_USERNAME")
	if adminUsername == "" {
		log.Fatal("ADMIN_USERNAME 环境变量未设置")
	}
	adminPassword = os.Getenv("ADMIN_PASSWORD")
	if adminPassword == "" {
		log.Fatal("ADMIN_PASSWORD 环境变量未设置")
	}
}

func setupRoutes() {
	// 公开访问
	http.HandleFunc("/", serveIndexPage)
	http.HandleFunc("/random-image", randomImageProxyHandler)
	http.HandleFunc("/api/random-image", randomImageAPIHandler)
	http.HandleFunc("/api/tags", tagsAPIHandler)

	// 本地图片静态文件服务
	localFileServer := http.FileServer(http.Dir(localImagesPath))
	http.Handle("/local/", http.StripPrefix("/local/", localFileServer))

	// 管理后台
	http.HandleFunc("/admin/login", adminLoginHandler)
	http.HandleFunc("/admin/logout", adminLogoutHandler)
	http.Handle("/admin", authMiddleware(http.HandlerFunc(adminDashboardHandler)))
	http.Handle("/admin/add", authMiddleware(http.HandlerFunc(adminAddImageHandler)))
	http.Handle("/admin/edit", authMiddleware(http.HandlerFunc(adminEditImageHandler)))
	http.Handle("/admin/delete", authMiddleware(http.HandlerFunc(adminDeleteImageHandler)))

	// 后台本地素材库管理
	http.Handle("/admin/local_files", authMiddleware(http.HandlerFunc(adminLocalFilesHandler)))
	http.Handle("/admin/download", authMiddleware(http.HandlerFunc(adminDownloadURLHandler)))
	http.Handle("/admin/rename_file", authMiddleware(http.HandlerFunc(adminRenameFileHandler)))
	http.Handle("/admin/delete_file", authMiddleware(http.HandlerFunc(adminDeleteFileHandler)))
}

// --- 数据库操作 ---

func initDB(ctx context.Context) error {
	_, err := dbpool.Exec(ctx, `CREATE TABLE IF NOT EXISTS images (id SERIAL PRIMARY KEY, url TEXT NOT NULL UNIQUE, tags TEXT[]);`)
	if err != nil {
		return fmt.Errorf("无法创建表: %w", err)
	}

	// 确保本地图片目录存在
	if err := os.MkdirAll(localImagesPath, os.ModePerm); err != nil {
		return fmt.Errorf("无法创建本地图片目录: %w", err)
	}

	var count int
	err = dbpool.QueryRow(ctx, "SELECT COUNT(*) FROM images").Scan(&count)
	if err != nil {
		return fmt.Errorf("无法查询表计数: %w", err)
	}
	if count > 0 {
		return nil
	}

	file, err := os.Open("image_urls.txt")
	if os.IsNotExist(err) {
		return nil
	}
	if err != nil {
		return fmt.Errorf("无法打开 image_urls.txt: %w", err)
	}
	defer file.Close()

	log.Println("正在从 image_urls.txt 向数据库迁移数据...")
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		parts := strings.Split(line, ",")
		url := strings.TrimSpace(parts[0])
		var tags []string
		if len(parts) > 1 {
			for _, tag := range parts[1:] {
				if trimmed := strings.TrimSpace(tag); trimmed != "" {
					tags = append(tags, trimmed)
				}
			}
		}
		_, err := dbpool.Exec(ctx, "INSERT INTO images (url, tags) VALUES ($1, $2) ON CONFLICT (url) DO NOTHING", url, tags)
		if err != nil {
			log.Printf("警告: 无法插入行 '%s': %v", line, err)
		}
	}
	log.Println("数据迁移完成。")
	return scanner.Err()
}

// --- 核心 API 和页面处理 ---

func chooseRandomImage(ctx context.Context, tagQuery string) (Image, error) {
	var img Image
	var err error
	if tagQuery == "" {
		query := `SELECT id, url, tags FROM images ORDER BY RANDOM() LIMIT 1`
		err = dbpool.QueryRow(ctx, query).Scan(&img.ID, &img.URL, &img.Tags)
	} else {
		query := `SELECT id, url, tags FROM images WHERE tags @> ARRAY[$1] ORDER BY RANDOM() LIMIT 1`
		err = dbpool.QueryRow(ctx, query, tagQuery).Scan(&img.ID, &img.URL, &img.Tags)
	}
	if err != nil {
		if err == pgx.ErrNoRows {
			return img, fmt.Errorf("没有找到匹配的图片")
		}
		return img, err
	}
	return img, nil
}

func randomImageAPIHandler(w http.ResponseWriter, r *http.Request) {
	tagQuery := r.URL.Query().Get("tag")
	img, err := chooseRandomImage(r.Context(), tagQuery)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	log.Printf("提供 API 数据 (标签: '%s'): ID %d, URL %s", tagQuery, img.ID, img.URL)
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
	json.NewEncoder(w).Encode(img)
}

func randomImageProxyHandler(w http.ResponseWriter, r *http.Request) {
	tagQuery := r.URL.Query().Get("tag")
	img, err := chooseRandomImage(r.Context(), tagQuery)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	log.Printf("提供图片 (标签: '%s'): %s", tagQuery, img.URL)

	// 如果是本地 URL，直接从文件服务器内部重定向或提供服务
	if strings.HasPrefix(img.URL, "/local/") {
		http.ServeFile(w, r, filepath.Join(localImagesPath, strings.TrimPrefix(img.URL, "/local/")))
		return
	}

	resp, err := httpClient.Get(img.URL)
	if err != nil {
		log.Printf("请求图床图片 %s 失败: %v", img.URL, err)
		http.Error(w, "无法获取图床图片", http.StatusInternalServerError)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		log.Printf("图床 %s 返回错误状态码: %d", img.URL, resp.StatusCode)
		http.Error(w, fmt.Sprintf("图床返回错误: %d", resp.StatusCode), http.StatusBadGateway)
		return
	}

	w.Header().Set("Content-Type", resp.Header.Get("Content-Type"))
	w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
	_, err = io.Copy(w, resp.Body)
	if err != nil {
		log.Printf("将图片流写入响应失败: %v", err)
	}
}

func serveIndexPage(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		http.NotFound(w, r)
		return
	}
	http.ServeFile(w, r, "index.html")
}

func tagsAPIHandler(w http.ResponseWriter, r *http.Request) {
	query := `SELECT DISTINCT unnest(tags) as tag FROM images ORDER BY tag;`
	rows, err := dbpool.Query(context.Background(), query)
	if err != nil {
		http.Error(w, "无法获取标签列表", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var tags []string
	for rows.Next() {
		var tag string
		if err := rows.Scan(&tag); err != nil {
			continue
		}
		tags = append(tags, tag)
	}

	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	json.NewEncoder(w).Encode(tags)
}

// --- 后台认证和中间件 ---

func authMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		cookie, err := r.Cookie("session_token")
		if err != nil {
			http.Redirect(w, r, "/admin/login", http.StatusFound)
			return
		}
		if !sessions[cookie.Value] {
			http.Redirect(w, r, "/admin/login", http.StatusFound)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func adminLoginHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method == http.MethodPost {
		r.ParseForm()
		if r.FormValue("username") == adminUsername && r.FormValue("password") == adminPassword {
			sessionToken := uuid.NewString()
			sessions[sessionToken] = true
			http.SetCookie(w, &http.Cookie{
				Name:    "session_token",
				Value:   sessionToken,
				Expires: time.Now().Add(12 * time.Hour),
				Path:    "/",
			})
			http.Redirect(w, r, "/admin", http.StatusFound)
			return
		}
	}
	pageData := PageData{Title: "登录"}
	templates.ExecuteTemplate(w, "login.html", pageData)
}

func adminLogoutHandler(w http.ResponseWriter, r *http.Request) {
	cookie, err := r.Cookie("session_token")
	if err == nil {
		delete(sessions, cookie.Value)
	}
	http.SetCookie(w, &http.Cookie{
		Name:   "session_token",
		Value:  "",
		MaxAge: -1,
		Path:   "/",
	})
	http.Redirect(w, r, "/admin/login", http.StatusFound)
}

// --- 后台 CRUD 操作 ---

func adminDashboardHandler(w http.ResponseWriter, r *http.Request) {
	rows, err := dbpool.Query(context.Background(), "SELECT id, url, tags FROM images ORDER BY id DESC")
	if err != nil {
		http.Error(w, "无法获取图片列表", http.StatusInternalServerError)
		return
	}
	defer rows.Close()
	var images []Image
	for rows.Next() {
		var img Image
		if err := rows.Scan(&img.ID, &img.URL, &img.Tags); err != nil {
			log.Printf("扫描图片数据失败: %v", err)
			continue
		}
		images = append(images, img)
	}
	pageData := PageData{
		Title:        "管理后台",
		UserLoggedIn: true,
		Images:       images,
	}
	templates.ExecuteTemplate(w, "dashboard.html", pageData)
}

func adminAddImageHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method == http.MethodPost {
		r.ParseForm()
		imgURL := r.FormValue("url")
		imageType := r.FormValue("image_type")
		otherTagsStr := r.FormValue("other_tags")

		var finalTags []string
		if imageType != "" {
			finalTags = append(finalTags, imageType)
		}
		for _, t := range strings.Split(otherTagsStr, ",") {
			if trimmed := strings.TrimSpace(t); trimmed != "" {
				finalTags = append(finalTags, trimmed)
			}
		}

		_, err := dbpool.Exec(context.Background(), "INSERT INTO images (url, tags) VALUES ($1, $2)", imgURL, finalTags)
		if err != nil {
			http.Error(w, "添加图片失败: "+err.Error(), http.StatusInternalServerError)
			return
		}
		http.Redirect(w, r, "/admin", http.StatusFound)
		return
	}

	// 预填充来自本地素材库的文件
	localFile := r.URL.Query().Get("local_file")
	img := Image{URL: "/local/" + localFile}

	pageData := PageData{
		Title:        "添加图片",
		UserLoggedIn: true,
		Data:         EditPageData{Image: img},
	}
	templates.ExecuteTemplate(w, "edit.html", pageData)
}

func adminEditImageHandler(w http.ResponseWriter, r *http.Request) {
	id := r.URL.Query().Get("id")
	if r.Method == http.MethodPost {
		r.ParseForm()
		imgURL := r.FormValue("url")
		imageType := r.FormValue("image_type")
		otherTagsStr := r.FormValue("other_tags")

		var finalTags []string
		if imageType != "" {
			finalTags = append(finalTags, imageType)
		}
		for _, t := range strings.Split(otherTagsStr, ",") {
			if trimmed := strings.TrimSpace(t); trimmed != "" {
				finalTags = append(finalTags, trimmed)
			}
		}

		_, err := dbpool.Exec(context.Background(), "UPDATE images SET url=$1, tags=$2 WHERE id=$3", imgURL, finalTags, id)
		if err != nil {
			http.Error(w, "更新图片失败: "+err.Error(), http.StatusInternalServerError)
			return
		}
		http.Redirect(w, r, "/admin", http.StatusFound)
		return
	}

	var img Image
	err := dbpool.QueryRow(context.Background(), "SELECT id, url, tags FROM images WHERE id=$1", id).Scan(&img.ID, &img.URL, &img.Tags)
	if err != nil {
		http.Error(w, "未找到该图片", http.StatusNotFound)
		return
	}

	data := EditPageData{Image: img}
	var otherTags []string
	for _, t := range img.Tags {
		if t == "desktop" {
			data.IsDesktop = true
		} else if t == "mobile" {
			data.IsMobile = true
		} else {
			otherTags = append(otherTags, t)
		}
	}
	data.OtherTags = strings.Join(otherTags, ", ")

	pageData := PageData{
		Title:        "编辑图片",
		UserLoggedIn: true,
		Data:         data,
	}
	templates.ExecuteTemplate(w, "edit.html", pageData)
}

func adminDeleteImageHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "无效的请求方法", http.StatusMethodNotAllowed)
		return
	}
	r.ParseForm()
	id := r.FormValue("id")
	_, err := dbpool.Exec(context.Background(), "DELETE FROM images WHERE id=$1", id)
	if err != nil {
		http.Error(w, "删除图片失败: "+err.Error(), http.StatusInternalServerError)
		return
	}
	http.Redirect(w, r, "/admin", http.StatusFound)
}

// --- 后台本地素材库操作 ---

func adminLocalFilesHandler(w http.ResponseWriter, r *http.Request) {
	files, err := os.ReadDir(localImagesPath)
	if err != nil {
		http.Error(w, "无法读取本地图片目录", http.StatusInternalServerError)
		return
	}

	var localFiles []LocalFile
	for _, file := range files {
		info, err := file.Info()
		if err == nil && !info.IsDir() {
			localFiles = append(localFiles, LocalFile{Name: file.Name(), ModTime: info.ModTime()})
		}
	}

	pageData := PageData{
		Title:        "本地素材库",
		UserLoggedIn: true,
		LocalFiles:   localFiles,
	}
	templates.ExecuteTemplate(w, "local_files.html", pageData)
}

func adminDownloadURLHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "无效请求", http.StatusMethodNotAllowed)
		return
	}
	r.ParseForm()
	fileURL := r.FormValue("url")
	if fileURL == "" {
		http.Error(w, "URL 不能为空", http.StatusBadRequest)
		return
	}

	resp, err := httpClient.Get(fileURL)
	if err != nil {
		http.Error(w, "下载失败: "+err.Error(), http.StatusInternalServerError)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		http.Error(w, fmt.Sprintf("下载失败，源站返回状态码: %d", resp.StatusCode), http.StatusInternalServerError)
		return
	}

	// 从 URL 解析文件名，如果无法解析则用 UUID
	parsedURL, err := url.Parse(fileURL)
	var fileName string
	if err == nil && filepath.Base(parsedURL.Path) != "." && filepath.Base(parsedURL.Path) != "/" {
		fileName = filepath.Base(parsedURL.Path)
	} else {
		fileName = uuid.NewString() + ".jpg" // 默认后缀
	}

	localPath := filepath.Join(localImagesPath, fileName)

	outFile, err := os.Create(localPath)
	if err != nil {
		http.Error(w, "无法在本地创建文件: "+err.Error(), http.StatusInternalServerError)
		return
	}
	defer outFile.Close()

	_, err = io.Copy(outFile, resp.Body)
	if err != nil {
		http.Error(w, "保存文件失败: "+err.Error(), http.StatusInternalServerError)
		return
	}

	http.Redirect(w, r, "/admin/local_files", http.StatusFound)
}

func adminRenameFileHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "无效请求", http.StatusMethodNotAllowed)
		return
	}
	r.ParseForm()
	oldName := r.FormValue("old_name")
	newName := r.FormValue("new_name")

	if oldName == "" || newName == "" || strings.Contains(newName, "/") {
		http.Error(w, "文件名不能为空且不能包含斜杠", http.StatusBadRequest)
		return
	}

	oldPath := filepath.Join(localImagesPath, oldName)
	newPath := filepath.Join(localImagesPath, newName)

	if err := os.Rename(oldPath, newPath); err != nil {
		http.Error(w, "重命名失败: "+err.Error(), http.StatusInternalServerError)
		return
	}

	http.Redirect(w, r, "/admin/local_files", http.StatusFound)
}

func adminDeleteFileHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "无效请求", http.StatusMethodNotAllowed)
		return
	}
	r.ParseForm()
	fileName := r.FormValue("file_name")
	if fileName == "" {
		http.Error(w, "文件名不能为空", http.StatusBadRequest)
		return
	}

	filePath := filepath.Join(localImagesPath, fileName)
	if err := os.Remove(filePath); err != nil {
		http.Error(w, "删除文件失败: "+err.Error(), http.StatusInternalServerError)
		return
	}

	http.Redirect(w, r, "/admin/local_files", http.StatusFound)
}

// --- HTML 模板 ---

func parseTemplates() {
	templates = template.New("").Funcs(template.FuncMap{
		"join": strings.Join,
	})
	template.Must(templates.ParseGlob("templates/*.html"))
}

