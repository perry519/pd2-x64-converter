#!/usr/bin/env node
import { readdirSync, readFileSync } from 'node:fs'
import { join, relative } from 'node:path'
import process from 'node:process'
import { fileURLToPath, URL } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const srcDir = join(root, 'src')
const files = []
const extensions = new Set(['.js', '.ts', '.svelte'])
const spacingPrefixes = [
  'min-w',
  'max-w',
  'w',
  'min-h',
  'max-h',
  'h',
  'p',
  'px',
  'py',
  'pt',
  'pr',
  'pb',
  'pl',
  'm',
  'mx',
  'my',
  'mt',
  'mr',
  'mb',
  'ml',
  'gap',
  'gap-x',
  'gap-y',
]
const spacingPattern = new RegExp(
  // The alternatives are complete Tailwind prefixes, not interchangeable characters.
  // eslint-disable-next-line regexp/prefer-character-class
  `(?<![\\w-])(${spacingPrefixes.join('|')})-\\[(\\d+(?:\\.\\d+)?)px\\]`,
  'g',
)
const colorPattern
  = /(?<![\w-])((?:[\w[\]().&/=!-]+:)*)((?:bg|text|border|fill|stroke|ring)-)(?:\(--([\w-]+)\)|\[var\(--([\w-]+)\)\])/g
const problems = []

walk(srcDir)

const colorAliases = readColorAliases()
for (const file of files) {
  const source = readFileSync(file, 'utf8')
  checkSpacing(file, source)
  checkColors(file, source, colorAliases)
}

if (problems.length > 0) {
  console.error('Non-canonical Tailwind classes found:')
  for (const problem of problems) {
    console.error(
      `${relative(root, problem.file)}:${problem.line}: ${problem.actual} -> ${problem.expected}`,
    )
  }
  process.exit(1)
}

function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    visitEntry(dir, entry)
  }
}

function visitEntry(dir, entry) {
  const path = join(dir, entry.name)
  if (entry.isDirectory()) {
    walkDirectory(path)
    return
  }
  if (hasCheckedExtension(path))
    files.push(path)
}

function walkDirectory(path) {
  if (relative(root, path) !== 'src/components/ui')
    walk(path)
}

function hasCheckedExtension(path) {
  return [...extensions].some(extension => path.endsWith(extension))
}

function checkSpacing(file, source) {
  for (const match of source.matchAll(spacingPattern)) {
    const [, prefix, pxText] = match
    const expected = `${prefix}-${spacingKey(Number(pxText))}`
    problems.push({
      file,
      line: lineNumber(source, match.index),
      actual: match[0],
      expected,
    })
  }
}

function checkColors(file, source, aliases) {
  for (const match of source.matchAll(colorPattern)) {
    const [, variants, utility, parenVar, bracketVar] = match
    const color = aliases.get(parenVar ?? bracketVar)
    if (!color)
      continue
    problems.push({
      file,
      line: lineNumber(source, match.index),
      actual: match[0],
      expected: `${variants}${utility}${color}`,
    })
  }
}

function readColorAliases() {
  const aliases = new Map()
  const css = readFileSync(join(root, 'src/app.css'), 'utf8')
  const pattern = /--color-([\w-]+):\s*var\(--([\w-]+)\);/g
  for (const [, color, variable] of css.matchAll(pattern)) {
    if (!aliases.has(variable))
      aliases.set(variable, color)
  }
  return aliases
}

function spacingKey(px) {
  if (px === 1)
    return 'px'
  return String(px / 4).replace(/\.0$/, '')
}

function lineNumber(source, index) {
  return source.slice(0, index).split('\n').length
}
