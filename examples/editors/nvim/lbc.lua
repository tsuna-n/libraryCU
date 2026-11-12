local M = {}

function M.explain(text)
  if not text:find('%S') then
    vim.notify('lbc: select some text or open a nonempty error log', vim.log.levels.WARN)
    return
  end
  local root = vim.g.lbc_project_root or vim.fn.getcwd(-1, 0)
  local ok, err = pcall(vim.system, {
    'lbc', 'explain', '--stdin', '--project', root, '--project-only', '--no-color',
  }, { stdin = text, text = true, cwd = root }, vim.schedule_wrap(function(result)
    local output = result.stdout or ''
    if result.code ~= 0 then
      output = ('lbc failed (exit %s, signal %s)\n%s\n%s'):format(
        result.code, result.signal, output, result.stderr or '')
    end
    local buffer = vim.api.nvim_create_buf(false, true)
    vim.bo[buffer].bufhidden = 'wipe'
    vim.api.nvim_buf_set_lines(buffer, 0, -1, false, vim.split(output, '\n', { plain = true }))
    vim.bo[buffer].modifiable = false
    vim.b[buffer].lbc_exit_code = result.code
    vim.cmd('botright 12split')
    vim.api.nvim_win_set_buf(0, buffer)
  end))
  if not ok then
    vim.notify('lbc: ' .. tostring(err), vim.log.levels.ERROR)
  end
end

function M.explain_selection()
  local mode = vim.fn.mode()
  if mode ~= 'v' and mode ~= 'V' and mode ~= '\22' then
    vim.notify('lbc: use the mapping from Visual mode', vim.log.levels.WARN)
    return
  end
  local lines = vim.fn.getregion(vim.fn.getpos('v'), vim.fn.getpos('.'), { type = mode })
  M.explain(table.concat(lines, '\n'))
end

vim.keymap.set('x', '<leader>le', M.explain_selection, { desc = 'lbc: Explain selection' })
vim.keymap.set('n', '<leader>le', function()
  M.explain(table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), '\n'))
end, { desc = 'lbc: Explain current log buffer' })

return M
