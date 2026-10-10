const CACHE_NAME = "xiuxian-v1";
const ASSETS = [
  "./index.html",
  "./manifest.json"
];

// 安装时缓存核心文件
self.addEventListener("install", (e) => {
  e.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(ASSETS))
  );
});

// 拦截请求（确保游戏可以正常加载）
self.addEventListener("fetch", (e) => {
  e.respondWith(
    caches.match(e.request).then((response) => {
      return response || fetch(e.request);
    })
  );
});

