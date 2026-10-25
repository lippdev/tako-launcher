// Configuração exclusiva da fixture: a WEB-03 aplica a política equivalente no React.
export default {
  cacheDir: "../../../target/web02a-vite-cache",
  server: {
    host: "127.0.0.1",
    port: 5192,
    strictPort: true,
    headers: {
      "Content-Security-Policy": "default-src http://127.0.0.1:5192; img-src http://127.0.0.1:5192 tako-img: data:; style-src http://127.0.0.1:5192 'unsafe-inline'; script-src http://127.0.0.1:5192; connect-src http://127.0.0.1:5192 ws://127.0.0.1:5192; object-src 'none'; base-uri 'none'; frame-src 'none'",
    },
  },
};
