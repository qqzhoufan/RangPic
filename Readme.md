## 1.切换到你想要的文件夹，比如/opt/randompic文件夹

``cd /opt/randompic``

没有这个文件夹可以使用创建

``mkdir /opt/randompic``

## 2.下载docker-compose.yaml文件

``wget https://raw.githubusercontent.com/qqzhoufan/RangPic/master/docker-compose.yaml``

## 3.手动创建图片列表txt文件

``nano image_urls.txt``

## 4.之后使用命令，验证创建的image_urls.txt是个文件

``ls -l image_urls.txt``

## 5.将自己的图片链接填入image_urls.txt中，一行一条链接即可

比如
```
https://blogsky.zhouwl.com/i/2025/05/26/683479325d5ba.webp
https://blogsky.zhouwl.com/i/2025/05/26/6834791e5e06d.webp
```

## 6.使用命令开始构建

``docker compose up -d``

## 7.默认17777端口，需要的可以自行修改文件中的端口

## 8.访问ip:端口号/random-image 即可生效,也可以通过域名解析

示例
``https://pic.19961216.xyz/random-image``
