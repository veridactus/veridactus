/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        veridactus: { primary: '#0a0e27', secondary: '#131633', accent: '#6c5ce7', success: '#00d4aa', warning: '#fdcb6e', error: '#ff7675' },
      },
    },
  },
  plugins: [],
}
