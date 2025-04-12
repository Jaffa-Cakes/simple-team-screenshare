import Mpegts from "mpegts.js";
import { WEBSOCKET_URL_BASE } from "./lib";
import { useEffect, useRef, useState } from "react";

export interface PlayerProps {
    stream_id: string;
    onClose: (streamId: string) => void;
}
  
export default function Player(props: PlayerProps) {
    const { stream_id } = props;
    const videoRef = useRef<HTMLVideoElement>(null);
    const playerRef = useRef<Mpegts.Player | null>(null);
    const [isHovering, setIsHovering] = useState(false);
    const [buttonIsHovering, setButtonIsHovering] = useState(false);

    useEffect(() => {
        if (Mpegts.isSupported() && videoRef.current) {
            const player = Mpegts.createPlayer({
                type: 'mpegts',
                url: `${WEBSOCKET_URL_BASE}/streams/${stream_id}`,
                isLive: true,
            }, {
                isLive: true,
                liveSync: true,
                liveSyncTargetLatency: 1.0,
                liveSyncMaxLatency: 2.0,
                liveSyncPlaybackRate: 1.5,
            });

            playerRef.current = player;
            player.attachMediaElement(videoRef.current);
            videoRef.current.muted = true; // Mute the video
            player.load();
            player.play();

            return () => {
                if (playerRef.current) {
                    playerRef.current.destroy();
                    playerRef.current = null;
                }
            };
        } else {
            console.error('MPEGTS.js is not supported or video element not found.');
        }
    }, []);

    const handleMouseEnter = () => {
        setIsHovering(true);
    };
    
      const handleMouseLeave = () => {
        setIsHovering(false);
    };

    const onClick = (event: any) => {
        event.stopPropagation();
        props.onClose(stream_id);
    }

    const fancyStreamId = String(stream_id).charAt(0).toUpperCase() + String(stream_id).slice(1);

    return (
        <div 
        className="relative"
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        >
            <div
            className={`absolute bottom-4 left-1/2 -translate-x-1/2 bg-black/30 text-white px-3 py-1 rounded-md transition-transform duration-300 ${isHovering && !buttonIsHovering ? '-translate-y-20' : '-translate-y-0/2'}`}
            >
                {fancyStreamId}
            </div>
            <video ref={videoRef} controls autoPlay></video>
            <button
            className={`absolute top-4 left-1/2 -translate-x-1/2 bg-red-600 hover:bg-red-700 cursor-pointer bg-opacity-50 text-white px-3 py-1 rounded-md transition-opacity duration-300 ${isHovering ? 'opacity-100' : 'opacity-0'}`}
            onClick={onClick}
            onMouseEnter={() => setButtonIsHovering(true)}
            onMouseLeave={() => setButtonIsHovering(false)}
            >
                Close
            </button>
        </div>
    );
}