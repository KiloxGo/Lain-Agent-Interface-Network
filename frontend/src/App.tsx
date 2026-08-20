import Chat from "./components/Chat";
import HealthCheck from "./components/HealthCheck";
import MemoryPanel from "./components/MemoryPanel";

function App() {
  return (
    <>
      <HealthCheck />
      <Chat />
      <MemoryPanel />
    </>
  );
}

export default App;
