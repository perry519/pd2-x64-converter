let appearance = 'system'

try {
  appearance = localStorage.getItem('pd2-x64-converter:appearance') ?? appearance
}
catch {}

const light = appearance === 'light'
  || (appearance === 'system' && matchMedia('(prefers-color-scheme: light)').matches)

document.documentElement.style.backgroundColor = light ? '#edf2f7' : '#0d1012'
