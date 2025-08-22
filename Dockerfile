FROM golang:1.24-alpine AS builder

WORKDIR /app

COPY go.mod ./
COPY go.sum ./

RUN go mod download

COPY cmd/rangpic/*.go ./

RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o /app/random-image-server .

FROM alpine:latest

WORKDIR /app

COPY --from=builder /app/random-image-server /app/random-image-server

COPY web/static/index.html /app/index.html

COPY data/image_urls.txt /app/image_urls.txt


EXPOSE 17777

ENTRYPOINT ["/app/random-image-server"]
