import tailwindcss from '@tailwindcss/vite';
import {sveltekit} from '@sveltejs/kit/vite';
import {defineConfig} from 'vite';
import {nodePolyfills} from "vite-plugin-node-polyfills";


export default defineConfig({
    plugins: [
        tailwindcss(),
        sveltekit(),
        nodePolyfills({
            exclude: [],
            globals: {
                Buffer: true,
                global: true,
                process: true,
            },
            protocolImports: true,
        })
    ],
});
