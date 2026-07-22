import { getCurrentWindow } from '@tauri-apps/api/window'
import { Popup } from './views/popup/Popup'
import { Manager } from './views/manager/Manager'
import { Settings } from './views/settings/Settings'

// Each Tauri window is labeled at creation time; the same bundled frontend
// renders a different view depending on which window it's running in.
function App() {
  const label = getCurrentWindow().label

  if (label === 'popup') return <Popup />
  if (label === 'settings') return <Settings />
  return <Manager />
}

export default App
