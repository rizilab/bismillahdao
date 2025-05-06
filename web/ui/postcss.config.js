// postcss.config.js
const env = process.env.NODE_ENV;
const isProd = env === "production";

const purgecss = require('@fullhuman/postcss-purgecss')({

    // Specify the paths to all of the template files in your project
    content: ['./src/*.rs'],

    keyframes: true,

    // Include any special characters you're using in this regular expression
    defaultExtractor: content => content.match(/[A-Za-z0-9-_:/]+/g) || []
})

module.exports = {
    plugins: [
        // ...(isProd ? [purgecss] : []),
        require('postcss-import'),
        require('tailwindcss')('tailwind.config.js'),
        require('autoprefixer'),
        require('postcss-nested'),
    ]
}