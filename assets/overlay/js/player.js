/**
 * Yambot Overlay - YouTube Player
 * Audio always plays through the browser (captured by OBS).
 * Video and title visibility are independently toggleable.
 */

let ytPlayer = null;
let progressRafId = null;
let playerVisible = true;
let pendingVolume = 50;

// Called automatically by YouTube IFrame API once it loads
function onYouTubeIframeAPIReady() {
    ytPlayer = new YT.Player('yt-iframe', {
        height: '180',
        width: '320',
        playerVars: {
            autoplay: 0,
            controls: 0,
            disablekb: 1,
            fs: 0,
            iv_load_policy: 3,
            modestbranding: 1,
            rel: 0,
            showinfo: 0,
        },
        events: {
            onReady: onPlayerReady,
            onStateChange: onPlayerStateChange,
        },
    });
}

function onPlayerReady() {
    ytPlayer.setVolume(pendingVolume);
}

function onPlayerStateChange(event) {
    if (event.data === YT.PlayerState.ENDED) {
        stopProgressBar();
        sendToBackend({ type: 'song_ended' });
    } else if (event.data === YT.PlayerState.PLAYING) {
        startProgressBar();
    } else {
        stopProgressBar();
    }
}

function playSong(videoId, title) {
    if (!ytPlayer || typeof ytPlayer.loadVideoById !== 'function') return;
    ytPlayer.loadVideoById(videoId);
    const titleEl = document.getElementById('player-title');
    if (titleEl) titleEl.textContent = title;
    resetProgressBar();
}

function stopPlayer() {
    if (ytPlayer && typeof ytPlayer.stopVideo === 'function') {
        ytPlayer.stopVideo();
    }
    stopProgressBar();
    resetProgressBar();
    const titleEl = document.getElementById('player-title');
    if (titleEl) titleEl.textContent = '';
}

function setPlayerVolume(volume) {
    pendingVolume = volume;
    if (ytPlayer && typeof ytPlayer.setVolume === 'function') {
        ytPlayer.setVolume(volume);
    }
}

function setVideoVisible(visible) {
    const wrapper = document.getElementById('player-video-wrapper');
    if (!wrapper) return;
    if (visible) {
        wrapper.classList.remove('video-hidden');
    } else {
        wrapper.classList.add('video-hidden');
    }
}

function setTitleVisible(visible) {
    const titleEl = document.getElementById('player-title');
    if (!titleEl) return;
    if (visible) {
        titleEl.classList.remove('title-hidden');
    } else {
        titleEl.classList.add('title-hidden');
    }
}

function setPlayerVisible(visible) {
    playerVisible = visible;
    const container = document.getElementById('player-container');
    if (!container) return;
    if (visible) {
        container.classList.remove('player-hidden');
    } else {
        container.classList.add('player-hidden');
    }
}

function startProgressBar() {
    stopProgressBar();
    function tick() {
        updateProgressBar();
        progressRafId = requestAnimationFrame(tick);
    }
    progressRafId = requestAnimationFrame(tick);
}

function stopProgressBar() {
    if (progressRafId !== null) {
        cancelAnimationFrame(progressRafId);
        progressRafId = null;
    }
}

function resetProgressBar() {
    const bar = document.getElementById('player-progress-fill');
    if (bar) bar.style.width = '0%';
}

function updateProgressBar() {
    if (!ytPlayer || typeof ytPlayer.getCurrentTime !== 'function') return;
    const current = ytPlayer.getCurrentTime();
    const duration = ytPlayer.getDuration();
    if (!duration || duration <= 0) return;
    const pct = Math.min(100, (current / duration) * 100);
    const bar = document.getElementById('player-progress-fill');
    if (bar) bar.style.width = pct + '%';
}

function pausePlayer() {
    if (ytPlayer && typeof ytPlayer.pauseVideo === 'function') {
        ytPlayer.pauseVideo();
    }
}

function resumePlayer() {
    if (ytPlayer && typeof ytPlayer.playVideo === 'function') {
        ytPlayer.playVideo();
    }
}

window.playSong = playSong;
window.stopPlayer = stopPlayer;
window.pausePlayer = pausePlayer;
window.resumePlayer = resumePlayer;
window.setPlayerVolume = setPlayerVolume;
window.setVideoVisible = setVideoVisible;
window.setTitleVisible = setTitleVisible;
window.setPlayerVisible = setPlayerVisible;
window.playerVisible = playerVisible;
