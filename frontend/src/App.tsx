import { useEffect, useRef, useState } from "react";
import { WEBSOCKET_URL_BASE } from "./lib";
import Player from "./Player";

export default function App() {
    const websocketRef = useRef<WebSocket | null>(null);
    const [streamList, setStreamList] = useState<string[]>([]);
    const [hiddenStreams, setHiddenStreams] = useState<string[]>([]);

    useEffect(() => {
        websocketRef.current = new WebSocket(`${WEBSOCKET_URL_BASE}/streams`);

        const websocket = websocketRef.current;

        if (!websocket) return;

        websocket.onopen = () => {
            console.log('WebSocket connection opened for stream list');
        };

        websocket.onclose = () => {
            console.error('WebSocket connection closed for stream list');
        };

        websocket.onerror = (error) => {
            console.error('WebSocket error:', error);
        };

        websocket.onmessage = (event) => {
            const data: string[] = JSON.parse(event.data);
            console.log('Received stream list:', data);
            setStreamList(data);
        };

        return () => {
            if (websocket.readyState === WebSocket.OPEN) {
                websocket.close();
            }
            websocket.onopen = null;
            websocket.onmessage = null;
            websocket.onclose = null;
            websocket.onerror = null;
            websocketRef.current = null;
        };
    }, []);

    return (
        <div className="min-h-screen bg-zinc-900 text-gray-200">
            <div className="flex flex-wrap min-h-screen items-center">
                {streamList.map((streamId) => {
                    if (hiddenStreams.includes(streamId)) {
                        return null;
                    }

                    return (
                        <div className={`${streamList.length - hiddenStreams.length > 1 ? "w-1/2" : ""}`}>
                            <Player 
                            key={streamId} 
                            stream_id={streamId}
                            onClose={(streamId) => setHiddenStreams((prev) => [...prev, streamId])}
                            />
                        </div>
                    )
                })}

                {hiddenStreams.length > 0 && (
                    <button
                    className="absolute bottom-4 left-1/2 -translate-x-1/2 bg-green-600/10 hover:bg-green-700/10 cursor-pointer text-white/80 px-3 py-1 rounded-md"
                    onClick={() => setHiddenStreams([])}
                    >
                        Show {hiddenStreams.length} Hidden Stream{hiddenStreams.length > 1 ? "s" : ""}
                    </button>
                )}
            </div>
        </div>
    );
}