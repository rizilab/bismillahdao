import commonjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import rust from "@wasm-tool/rollup-plugin-rust";
import copy from 'rollup-plugin-copy';

export default {
    input: {
        web_ui: "./Cargo.toml",
    },
    output: {
        dir: "dist/js",
        format: "iife",
        sourcemap: true,
        chunkFileNames: "[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
    },
    plugins: [
        rust({
            verbose: false,
            optimize: {
                release: true,
                rustc: true,
            },
        }),
        nodeResolve(),
        commonjs(),
        copy({
            targets: [
                { src: 'devhtml/index.html', dest: 'dist' },
                { src: 'devhtml/public', dest: 'dist/' },
                { src: 'R4gmiOidcTrustedDomains.js', dest: 'dist/js/OidcTrustedDomains.js' }
            ]
        })
    ],
};