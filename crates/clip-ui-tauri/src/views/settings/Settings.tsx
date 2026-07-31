import { useEffect, useState } from 'react'
import { callCommand } from '../../state/client'
import type { AppSettings, DiagnosticsReport, Rule } from '../../state/types'

function newRuleId(): string {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID() : `rule-${Date.now()}-${Math.random()}`
}

export function Settings() {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [hotkeyInput, setHotkeyInput] = useState('')
  const [hotkeyError, setHotkeyError] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)
  const [diagnostics, setDiagnostics] = useState<DiagnosticsReport | null>(null)
  const [rules, setRules] = useState<Rule[]>([])
  const [newRuleAppMatch, setNewRuleAppMatch] = useState('')

  useEffect(() => {
    void callCommand<AppSettings>('get_settings').then((loaded) => {
      setSettings(loaded)
      setHotkeyInput(loaded.hotkey_binding)
    })
    void callCommand<DiagnosticsReport>('get_diagnostics').then(setDiagnostics)
    void callCommand<Rule[]>('list_rules').then(setRules)
  }, [])

  const handleSaveHotkey = async () => {
    if (!settings) return
    setHotkeyError(null)
    setSaved(false)
    const updated: AppSettings = { ...settings, hotkey_binding: hotkeyInput }
    try {
      await callCommand('update_settings', { settings: updated })
      setSettings(updated)
      setSaved(true)
    } catch (error) {
      setHotkeyError(String(error))
    }
  }

  const handleCreateRule = async () => {
    const rule: Rule = {
      id: newRuleId(),
      app_match: newRuleAppMatch,
      window_match: null,
      mime_match: null,
      action: 'exclude',
      enabled: true,
    }
    await callCommand('save_rule', { rule })
    setRules((existing) => [...existing, rule])
    setNewRuleAppMatch('')
  }

  const handleToggleRule = async (rule: Rule) => {
    const updated: Rule = { ...rule, enabled: !rule.enabled }
    await callCommand('save_rule', { rule: updated })
    setRules((existing) => existing.map((r) => (r.id === rule.id ? updated : r)))
  }

  const handleDeleteRule = async (id: string) => {
    await callCommand('delete_rule', { id })
    setRules((existing) => existing.filter((rule) => rule.id !== id))
  }

  const handleClose = async () => {
    await callCommand('close_settings_window')
  }

  return (
    <div className="settings">
      <section>
        <h2>Hotkey</h2>
        <label>
          Hotkey binding
          <input aria-label="Hotkey binding" value={hotkeyInput} onChange={(event) => setHotkeyInput(event.target.value)} />
        </label>
        <button type="button" onClick={handleSaveHotkey}>
          Save hotkey
        </button>
        {hotkeyError && <p role="alert">{hotkeyError}</p>}
        {saved && <p>Saved</p>}
      </section>

      <section>
        <h2>Diagnostics</h2>
        {diagnostics && (
          <ul>
            {Object.entries(diagnostics.capabilities).map(([capability, supported]) => (
              <li key={capability}>
                {capability}: {supported ? 'Supported' : 'Unsupported'}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h2>Rules</h2>
        <label>
          New rule app match
          <input
            aria-label="New rule app match"
            value={newRuleAppMatch}
            onChange={(event) => setNewRuleAppMatch(event.target.value)}
          />
          <span className="hint">e.g. 1Password</span>
        </label>
        <button type="button" onClick={handleCreateRule}>
          Add rule
        </button>
        <ul>
          {rules.map((rule) => (
            <li key={rule.id}>
              {rule.app_match}
              <button type="button" aria-label={`Toggle rule ${rule.id}`} onClick={() => handleToggleRule(rule)}>
                {rule.enabled ? 'Disable' : 'Enable'}
              </button>
              <button type="button" aria-label={`Delete rule ${rule.id}`} onClick={() => handleDeleteRule(rule.id)}>
                Delete rule
              </button>
            </li>
          ))}
        </ul>
      </section>

      <div className="settings-footer">
        <button type="button" onClick={handleClose}>
          Close
        </button>
      </div>
    </div>
  )
}
