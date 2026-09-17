# frozen_string_literal: true

# Test-only process harness. The production adapter will use the generated
# ParserHost interface; this script is not a second public wire protocol.
# No merge, matching, ownership, or rendering decisions belong here.
require "json"
require "psych"

# Psych versions differ on whether a leading BOM counts as a character column.
# Probe the native AST convention without changing the document passed to Psych.
PSYCH_BOM_COLUMN_WIDTH = Psych.parse_stream("\uFEFFx: 1\n").children.first.start_column
raise "unsupported Psych BOM column convention" unless [0, 1].include?(PSYCH_BOM_COLUMN_WIDTH)

def project(request)
  source = request.fetch("source")
  text = source.fetch("bytes").pack("C*").force_encoding(Encoding::UTF_8)
  raise "test adapter does not yet support bare CR line coordinates" if source.fetch("descriptor").fetch("line_endings").fetch("bare_cr").positive?
  lines = text.split("\n", -1)
  bom_bytes = text.start_with?("\uFEFF") ? 3 : 0
  lines[0] = lines[0].delete_prefix("\uFEFF") if bom_bytes.positive?
  starts = [0]
  text.bytes.each_with_index { |byte, index| starts << index + 1 if byte == 10 }
  offset = lambda do |row, column|
    # Psych's columns count Unicode characters, not UTF-8 bytes.
    # Libyaml reports the next row at EOF even without a final newline.
    next text.bytesize if row == lines.length && column.zero?
    column -= PSYCH_BOM_COLUMN_WIDTH if row.zero? && bom_bytes.positive?
    line = lines.fetch(row)
    raise "invalid character column: row=#{row} column=#{column} length=#{line.length}" if column.negative? || column > line.length
    starts.fetch(row) + (row.zero? ? bom_bytes : 0) + line[0, column].bytesize
  end
  point = lambda do |byte|
    row = starts.bsearch_index { |start| start > byte }
    row = row ? row - 1 : starts.length - 1
    {row: row, column: byte - starts.fetch(row)}
  end
  nodes = []
  visit = nil
  visit = lambda do |node, parent|
    id = "n#{nodes.length}"
    kind = node.class.name.split("::").last.downcase
    first = kind == "stream" ? 0 : offset.call(node.start_line, node.start_column)
    last = kind == "stream" ? text.bytesize : offset.call(node.end_line, node.end_column)
    facts = {}
    %i[value style plain quoted anchor tag implicit implicit_end].each do |attribute|
      facts[attribute] = node.public_send(attribute) if node.respond_to?(attribute)
    end
    record = {
      id: id, type: kind, native_type: node.class.name, role: "structural",
      named: true, missing: false, has_error: false,
      span: {range: {start_byte: first, end_byte: last}, start_point: point.call(first), end_point: point.call(last)},
      parent_id: parent, children: [], semantic_roles: [], unsupported_features: [],
      extensions: [{schema: "structuredmerge.extension/ruby-psych/v1", namespace: "ruby-psych", capabilities: [], payload: facts}],
      metadata: {}
    }
    nodes << record
    (node.children || []).each_with_index do |child, index|
      field = kind == "mapping" ? (index.even? ? "key" : "value") : nil
      record[:children] << {node_id: visit.call(child, id), index: index, field_name: field}
    end
    id
  end
  root = visit.call(Psych.parse_stream(text), nil)
  {request_id: request.fetch("request_id"), source: source.fetch("descriptor"), ok: true,
   root_id: root, nodes: nodes, comments: [], diagnostics: [], extensions: [], metadata: {}}
rescue Psych::SyntaxError => error
  {request_id: request.fetch("request_id"), source: source.fetch("descriptor"), ok: false,
   root_id: nil, nodes: [], comments: [], extensions: [], metadata: {}, diagnostics: [{
     id: "psych.syntax", severity: "error", category: "parse_error", code: "psych.syntax",
     message: error.problem, source_role: source.fetch("descriptor").fetch("role"),
     span: nil, node_id: nil, blocking: true, metadata: {}
   }]}
end

if $PROGRAM_NAME == __FILE__
  STDOUT.write(JSON.generate(JSON.parse(STDIN.read).map { |request| project(request) }))
end
