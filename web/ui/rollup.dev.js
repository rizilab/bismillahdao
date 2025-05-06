import commonjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import rust from "@wasm-tool/rollup-plugin-rust";
import copy from 'rollup-plugin-copy';

export default {
    input: {
        landing: './landing/Cargo.toml',
        auth: './auth/Cargo.toml',
        
    },
    output: {
        dir: "dist/js",
        sourcemap: true,
        chunkFileNames: "[name].js",
        assetFileNames: "assets/[name][extname]"
    },
    plugins: [
        rust({
            verbose: true,
            optimize: {
                release: false,
                rustc: true
            },
            extraArgs: {
                cargo: ["--features", "develop", "--target", "wasm32-unknown-unknown"]
            }
        }),
        copy({
            targets: [
                { src: 'devhtml/index.html', dest: 'dist' },
                { src: 'devhtml/public', dest: 'dist/' },
                { src: 'R4gmiOidcTrustedDomains.js', dest: 'dist/js/OidcTrustedDomains.js' }
            ]
        }),
        nodeResolve(),
        commonjs(),
    ],
};