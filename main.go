package main

import (
	"bufio"
	"fmt"
	"io"
	"log"
	"math/rand"
	"net/http"
	"os"
	"strings"
	"time"
)

// imageURLs 将从文件中加载
var imageURLs []string

const urlFilePath = "image_urls.txt" // 存放图片链接的文本文件名

func main() {
	// 初始化随机数种子
	rand.Seed(time.Now().UnixNano())

	// 从文件加载图片链接
	var err error
	imageURLs, err = loadImageURLsFromFile(urlFilePath)
	if err != nil {
		log.Fatalf("无法从文件 %s 加载图片链接: %v", urlFilePath, err)
	}

	if len(imageURLs) == 0 {
		log.Fatalf("文件 %s 中没有找到有效的图片链接，或者文件为空。", urlFilePath)
	}

	log.Printf("成功从 %s 加载了 %d 个图片链接。\n", urlFilePath, len(imageURLs))

	// 设置路由
	http.HandleFunc("/random-image", randomImageProxyHandler)

	// 启动 HTTP 服务器
	port := "17777"
	log.Printf("服务器启动在 http://localhost:%s/random-image\n", port)
	log.Fatal(http.ListenAndServe(":"+port, nil))
}

// loadImageURLsFromFile 从指定的文本文件中加载图片链接，每行一个链接
func loadImageURLsFromFile(filePath string) ([]string, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return nil, fmt.Errorf("打开文件 %s 失败: %w", filePath, err)
	}
	defer file.Close()

	var urls []string
	scanner := bufio.NewScanner(file)
	lineNumber := 0
	for scanner.Scan() {
		lineNumber++
		line := strings.TrimSpace(scanner.Text())
		// 忽略空行和注释行 (以 # 开头)
		if line != "" && !strings.HasPrefix(line, "#") {
			urls = append(urls, line)
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("读取文件 %s 时发生错误: %w", filePath, err)
	}

	return urls, nil
}

// randomImageProxyHandler 处理随机图片请求，从图床代理图片
func randomImageProxyHandler(w http.ResponseWriter, r *http.Request) {
	if len(imageURLs) == 0 {
		// 这个检查理论上在 main 函数已经覆盖，但作为双重保险
		http.Error(w, "没有可用的图片链接", http.StatusNotFound)
		return
	}

	// 随机选择一个图片链接
	randomIndex := rand.Intn(len(imageURLs))
	randomImageURL := imageURLs[randomIndex]

	log.Printf("尝试从图床获取图片: %s\n", randomImageURL)

	// 创建一个 HTTP 客户端，可以设置超时等
	client := http.Client{
		Timeout: 15 * time.Second, // 设置请求超时，例如15秒
	}

	// 向图片链接发起 GET 请求
	resp, err := client.Get(randomImageURL)
	if err != nil {
		log.Printf("请求图床图片 %s 失败: %v\n", randomImageURL, err)
		http.Error(w, "无法获取图床图片", http.StatusInternalServerError)
		return
	}
	defer resp.Body.Close()

	// 检查图床的响应状态码
	if resp.StatusCode != http.StatusOK {
		log.Printf("图床 %s 返回错误状态码: %d\n", randomImageURL, resp.StatusCode)
		http.Error(w, fmt.Sprintf("图床返回错误: %d", resp.StatusCode), http.StatusBadGateway)
		return
	}

	// 设置我们自己 API 的响应头
	w.Header().Set("Content-Type", "image/webp") // 假设所有链接都是 WebP
	w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
	w.Header().Set("Pragma", "no-cache")
	w.Header().Set("Expires", "0")

	// 将从图床获取的图片内容直接流式传输到客户端
	_, err = io.Copy(w, resp.Body)
	if err != nil {
		log.Printf("将图片流写入响应失败: %v\n", err)
	} else {
		log.Printf("成功代理并提供了图片: %s\n", randomImageURL)
	}
}
