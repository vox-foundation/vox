export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        void: '#0a0a0f', steel: '#8b9db5', brass: '#c9a84c',
        cyan: '#00e5ff', border: '#1e2a3a', foreground: '#d4dce8', primary: '#00e5ff',
      },
      fontFamily: {
        rajdhani: ['Rajdhani', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
};
