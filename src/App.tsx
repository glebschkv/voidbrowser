import "./styles/global.css";
import { NavigationControls } from "./components/browser/NavigationControls";
import { AddressBar } from "./components/browser/AddressBar";

function App() {
  return (
    <div class="h-[50px] bg-neutral-800 border-b border-neutral-700 flex items-center px-2">
      <NavigationControls />
      <AddressBar />
    </div>
  );
}

export default App;
