import { useEffect, useState } from "react";

function HealthCheck() {
  const [status, setStatus] = useState("checking...");

  useEffect(() => {
    fetch("/api/health")
      .then((res) => res.json())
      .then((data) => setStatus(data.status))
      .catch((err) => setStatus("error:" + err.message));
  }, []);

  return <div>Backend Status:{status}</div>;
}

export default HealthCheck;
