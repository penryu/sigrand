FROM golang:alpine AS build
WORKDIR /build
COPY . .
RUN go build

FROM alpine
WORKDIR /app
COPY start.sh .
COPY sigfile /root/.sigfile
COPY --from=build /build/sigrand .
CMD ["./start.sh"]
