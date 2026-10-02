/* 由 `ham-web-tools postbuild` 生成，请勿手动修改。模板：crates/tools/templates/sw.js */
"use strict";

const PRECACHE_VERSION = "__CACHE_VERSION__";
const PRECACHE = `precache-${PRECACHE_VERSION}`;
const PRECACHE_MANIFEST = __PRECACHE_MANIFEST__;
const PRECACHE_URLS = new Set(PRECACHE_MANIFEST.map((e) => e.url));

const RUNTIME = {
  // 题库 JSON：优先网络保证新鲜度，离线时回退缓存
  questions: { name: "questions-json", maxEntries: 10, maxAgeSeconds: 7 * 24 * 60 * 60 },
  // 题目图片：优先缓存，同时后台更新
  images: { name: "question-images", maxEntries: 300, maxAgeSeconds: 30 * 24 * 60 * 60 },
};
const TS_HEADER = "x-sw-cached-at";

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(PRECACHE)
      .then((cache) => cache.addAll(PRECACHE_MANIFEST.map((e) => new Request(e.url, { cache: "reload" })))),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(keys.filter((k) => k.startsWith("precache-") && k !== PRECACHE).map((k) => caches.delete(k)));
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("message", (event) => {
  const type = event.data && event.data.type;
  if (type === "SKIP_WAITING" || type === "skip-waiting") self.skipWaiting();
});

async function stamp(response) {
  const headers = new Headers(response.headers);
  headers.set(TS_HEADER, String(Date.now()));
  const body = await response.blob();
  return new Response(body, { status: response.status, statusText: response.statusText, headers });
}

function isFresh(response, maxAgeSeconds) {
  const ts = Number(response.headers.get(TS_HEADER));
  return !ts || Date.now() - ts <= maxAgeSeconds * 1000;
}

async function trim(cache, maxEntries) {
  const keys = await cache.keys();
  const extra = keys.length - maxEntries;
  for (let i = 0; i < extra; i++) await cache.delete(keys[i]);
}

async function putRuntime(cfg, request, response) {
  const cache = await caches.open(cfg.name);
  await cache.put(request, await stamp(response));
  await trim(cache, cfg.maxEntries);
}

async function matchRuntime(cfg, request, options) {
  const cache = await caches.open(cfg.name);
  const hit = await cache.match(request, options);
  if (hit && !isFresh(hit, cfg.maxAgeSeconds)) {
    await cache.delete(request, options);
    return undefined;
  }
  return hit;
}

async function matchPrecache(request, options) {
  const cache = await caches.open(PRECACHE);
  return cache.match(request, options);
}

async function networkFirst(event, cfg) {
  const request = event.request;
  try {
    const response = await fetch(request);
    if (response.ok) event.waitUntil(putRuntime(cfg, request, response.clone()));
    return response;
  } catch (err) {
    const cached =
      (await matchRuntime(cfg, request)) ||
      (await matchRuntime(cfg, request, { ignoreSearch: true })) ||
      (await matchPrecache(request, { ignoreSearch: true }));
    if (cached) return cached;
    throw err;
  }
}

async function staleWhileRevalidate(event, cfg) {
  const request = event.request;
  const cached = (await matchRuntime(cfg, request)) || (await matchPrecache(request));
  const update = fetch(request).then(async (response) => {
    if (response.ok) await putRuntime(cfg, request, response.clone());
    return response;
  });
  if (cached) {
    event.waitUntil(update.catch(() => undefined));
    return cached;
  }
  return update;
}

async function navigation(event) {
  try {
    return await fetch(event.request);
  } catch (err) {
    const shell = (await matchPrecache("/index.html")) || (await matchPrecache("/"));
    if (shell) return shell;
    throw err;
  }
}

async function precacheFirst(event, path) {
  return (await matchPrecache(path)) || fetch(event.request);
}

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  if (/^\/questions\/.*\.json$/.test(url.pathname)) {
    event.respondWith(networkFirst(event, RUNTIME.questions));
  } else if (/^\/questions\/images\/.*\.(?:png|jpg|jpeg|gif|webp|svg)$/i.test(url.pathname)) {
    event.respondWith(staleWhileRevalidate(event, RUNTIME.images));
  } else if (request.mode === "navigate") {
    event.respondWith(navigation(event));
  } else if (PRECACHE_URLS.has(url.pathname) && !url.search) {
    event.respondWith(precacheFirst(event, url.pathname));
  }
});

// —— Web Push：收到服务端推送时显示系统通知，点击后回到应用 ——
self.addEventListener("push", (event) => {
  let payload = {};
  try {
    payload = event.data ? event.data.json() : {};
  } catch (err) {
    payload = { title: "业余无线电", body: event.data ? event.data.text() : "" };
  }
  const title = payload.title || "业余无线电";
  const options = {
    body: payload.body || "",
    icon: "/pwa-icon-192.png",
    badge: "/pwa-icon-192.png",
    tag: payload.tag || "ham-exam",
    data: { url: payload.url || "/" },
  };
  event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const url = (event.notification.data && event.notification.data.url) || "/";
  event.waitUntil(
    self.clients
      .matchAll({ type: "window", includeUncontrolled: true })
      .then((list) => {
        for (const client of list) {
          if ("focus" in client) {
            client.focus();
            return client;
          }
        }
        return self.clients.openWindow(url);
      }),
  );
});
