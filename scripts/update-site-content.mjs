import fs from 'node:fs'
import path from 'node:path'
import {fileURLToPath} from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const sitePath = path.join(repoRoot, 'site', 'index.html')

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

function firstCodeBlockAfter(markdown, heading) {
  const start = markdown.indexOf(heading)
  if (start === -1) throw new Error(`Missing documentation heading: ${heading}`)
  const match = markdown.slice(start + heading.length).match(/```[^\n]*\n([\s\S]*?)```/)
  if (!match) throw new Error(`Missing documentation code block after: ${heading}`)
  return match[1].trim()
}

function codeBlockForLanguage(markdown, language) {
  return firstCodeBlockAfter(markdown, `\n\n${language}:`)
}

function firstParagraphAfter(markdown, heading) {
  const start = markdown.indexOf(heading)
  if (start === -1) throw new Error(`Missing authored heading: ${heading}`)
  const afterHeading = markdown.slice(start + heading.length).replace(/^[^\n]*\n/, '')
  const paragraph = afterHeading
    .split(/\n\s*\n/)
    .map((part) => part.trim())
    .find((part) => part && !part.startsWith('#') && !part.startsWith('```'))
  if (!paragraph) throw new Error(`Missing authored paragraph after: ${heading}`)
  return paragraph.replaceAll('\n', ' ')
}

function assertSame(label, expected, actual) {
  if (expected !== actual) {
    throw new Error(`${label} is out of sync with docs/authored/`)
  }
}

function escapeHtml(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

function replaceCode(html, id, value) {
  const pattern = new RegExp(`(<code id="${id}">)[\\s\\S]*?(</code>)`)
  if (!pattern.test(html)) throw new Error(`Missing site code block: ${id}`)
  return html.replace(pattern, `$1${escapeHtml(value)}$2`)
}

const demoValues = {
  toystory: 'eye-candy',
  woody: 'dua',
  '"Woody"': '"Dua"',
  'andys-room': 'underground-station',
  '"Andy\'s Room"': '"Underground Station"',
  jessie: 'dua',
  '"Jessie"': '"Dua"',
  'pizza-planet': 'dua-orbit-station',
  '25th-chapter': 'eye-candy',
  'nathan-gunn': 'dua',
}

function websiteExample(command) {
  return Object.entries(demoValues).reduce(
    (result, [from, to]) => result.replaceAll(from, to),
    command,
  )
}

const initCommand = read('docs/generated/commands/init.md')
const createCommand = read('docs/generated/commands/create.md')
const addCommand = read('docs/generated/commands/add.md')
const presignCommand = read('docs/generated/commands/presign.md')
const mcpOverview = read('docs/authored/mcp/overview.md')
const readme = read('README.md')

const install = firstCodeBlockAfter(readme, '### Installation Script')
const initialize = websiteExample(firstCodeBlockAfter(initCommand, '## Examples'))
const create = websiteExample(firstCodeBlockAfter(createCommand, '## Examples'))
const add = websiteExample(firstCodeBlockAfter(addCommand, '## Examples'))
const typescriptSdk = websiteExample(codeBlockForLanguage(presignCommand, 'TypeScript'))
const pythonSdk = websiteExample(codeBlockForLanguage(presignCommand, 'Python'))

let html = read(path.relative(repoRoot, sitePath))
html = html.replace(/\s*<meta name="px-(?:mcp-summary|skills-install)"[^>]*\/>/g, '')
html = html.replace(/\s*<script[^>]*>[\s\S]*?base\.href=\"\/px\/\"[\s\S]*?<\/script>/g, '')
html = html.replace('1. Install px + skills', '1. Install PX + skills')
html = html.replace('1. Install a PX dependency', '1. Install PX + skills')
html = html.replace('a. Connect MCP with Codex', '2. Initialize a repository')
html = html.replace('2. Start the Eye-Candy world', '3. Create an entity')
html = html.replace('TypeScript SDK</p>', 'TypeScript SDK: presign</p>')
html = html.replace('Python SDK</p>', 'Python SDK: presign</p>')
html = html.replace('3. Add a representation', '4. Add a representation')
html = html.replace(
  '<span class="underline decoration-2 underline-offset-4">Explore Portals</span>',
  '<a href="https://portals.works" class="underline decoration-2 underline-offset-4">Explore Portals</a>',
)
html = replaceCode(html, 'px-code-1', install)
html = replaceCode(html, 'px-code-2', initialize)
html = replaceCode(html, 'px-code-3', create)
html = replaceCode(html, 'px-code-4', add)
html = replaceCode(html, 'px-code-5', typescriptSdk)
html = replaceCode(html, 'px-code-6', pythonSdk)
const mcpSummary = escapeHtml(firstParagraphAfter(mcpOverview, '## MCP Server'))
// Keep the authored MCP overview in sync with the public README. CLI/SDK
// examples come from generated command docs; install comes from the README.
assertSame(
  'README MCP summary',
  firstParagraphAfter(mcpOverview, '## MCP Server'),
  firstParagraphAfter(readme, '## MCP Server (mandatory for agents)'),
)
const runtimeBase = `<script data-px-base>(function(){if(location.hostname==="portals.works"||location.hostname==="www.portals.works"){var base=document.createElement("base");base.href="/px/";document.head.insertBefore(base,document.head.firstChild);}})();</script>`
html = html.replace('<link rel="icon"', `${runtimeBase}\n  <link rel="icon"`)
html = html.replace('</head>', `  <meta name="px-mcp-summary" content="${mcpSummary.replaceAll('"', '&quot;')}" />\n</head>`)
fs.writeFileSync(sitePath, html)

console.log('Updated PX site examples from README and generated command docs.')
