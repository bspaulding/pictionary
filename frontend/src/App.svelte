<script>
	let room;
	let inputRoom;
	let createRoomFailed = false;
	let currentWord;
	let socket;
	let paths = [[]];
	let roomNotFound = false;
	function closePath() {
		paths = [...paths, []];
	}
	function pointCreated(point) {
		paths = [...paths.slice(0, paths.length - 1), [...paths[paths.length - 1], point]];
	}

	function handleAction(action) {
		console.log('Handling action: ', action);
		switch (action.type) {
			case 'pathClosed':
				closePath();
				return;
			case 'pointCreated':
				pointCreated(action.payload);
				return;
			case 'pathSet':
				paths = action.payload;
				return;
			case 'skipWordCompleted':
				paths = [[]];
				currentWord = action.payload;
				return;
			case 'newWord':
				paths = [[]];
				currentWord = action.payload;
				return;
			case 'RoomNotFound':
				roomNotFound = true;
				return;
			case 'nextRound':
				currentWord = undefined;
				return;
			case 'clearPaths':
				paths = [[]];
				return;
		}
	}

	function socketError(e) {
		console.log("socket error:", e);
	}

	function socketClose() {
		console.log("socket closed");
		room = undefined;
		paths = [[]];
	}

	let isSocketOpen = false;
	function socketOpen() {
		console.log('socket open');
		isSocketOpen = true;
	}

	function sendIfOpen(event) {
		if (!isSocketOpen) {
			return;
		}

		const action = event.payload ? { [event.type]: event.payload } : event.type;
		console.log("Sending action: ", action);
		socket.send(JSON.stringify(action));
	}

	function joinInputRoom() {
		joinRoom(inputRoom);
	}

	function socketMessage(event) {
		const actions = JSON.parse(event.data);
		console.log("Received actions: ", actions);
		typeof actions === "string"
			? handleAction({ type: actions })
			: Object.entries(actions).map(entry => handleAction({ type: entry[0], payload: entry[1] }));
	}

	const location = window.location;
	// const location = { protocol: 'https:', host: 'pictionary.motingo.com', port: '3000' };

	async function joinRoom(roomToJoin) {
			roomNotFound = false;
			socket = new WebSocket(`${location.protocol === "http:" ? "ws" : "wss"}://${location.host}/api/v1/rooms/${roomToJoin}/ws`);
			socket.onerror = socketError;
			socket.onclose = socketClose;
			socket.onopen = () => {
				room = roomToJoin;
				inputRoom = '';
				socketOpen();
			};
			socket.onmessage = socketMessage;
			console.log(socket);
	}

	async function createRoom() {
		createRoomFailed = false;
		const response = await fetch(`${location.protocol}//${location.host}/api/v1/rooms`, { method: "POST" });
		if (response.ok) {
			const json = await response.json();
			room = json.room;
			joinRoom(room);
		} else {
			createRoomFailed = true;
		}
	}

	let isDrawing = false;
	function boardMouseDown(e) {
		if (!currentWord) {
			return;
		}
		e.preventDefault();
		e.stopImmediatePropagation();
		isDrawing = true;
	}
	function boardMouseUp(e) {
		if (!currentWord) {
			return;
		}
		e.preventDefault();
		e.stopImmediatePropagation();
		isDrawing = false;
		const action = { type: 'pathClosed' };
		handleAction(action);
		sendIfOpen(action);
	}
	function boardMouseMove(e) {
		if (!currentWord) {
			return;
		}
		e.preventDefault();
		e.stopImmediatePropagation();
		if (isDrawing) {
			const point = { x: e.offsetX, y: e.offsetY };
			const action = { type: 'pointCreated', payload: point };
			handleAction(action);
			sendIfOpen(action);
		}
	}
	function boardTouchMove(e) {
		if (!currentWord) {
			return;
		}
		e.preventDefault();
		e.stopImmediatePropagation();
		if (isDrawing) {
			const touch = e.changedTouches[0];
			const rect = document.querySelector("svg").getBoundingClientRect()
			console.log({ rect, touch});
			const { top, left } = rect;
			const point = { x: Math.max(0, Math.round(touch.pageX - left - window.scrollX)), y: Math.max(Math.round(touch.pageY - top - window.scrollY)) };
			const action = { type: 'pointCreated', payload: point };
			handleAction(action);
			sendIfOpen(action);
		}
	}

	function skipWord() {
		sendIfOpen({ type: 'skipWordStart' });
	}

	function nextRound() {
		handleAction({ type: 'nextRound' });
		sendIfOpen({ type: 'nextRound' });
	}

	function clearBoard() {
		handleAction({ type: 'clearPaths' });
		sendIfOpen({ type: 'clearPaths' });
	}
</script>

<main>
	<h1>Pictionary</h1>
	{#if room}
		<div class="row space-between">
		<p>ROOM<br/><span class="room">{room}</span></p>
		{#if currentWord}
		<p>Your turn! The word is<br/>
		<span class="current-word">{currentWord}<span></p>
		{/if}
		</div>
		{#if currentWord}
		<div class="row">
		<button on:click={skipWord}>
			Skip Word
		</button>
		<button on:click={nextRound}>
			Next Round
		</button>
		<button on:click={clearBoard}>
			Clear
		</button>
		</div>
		{/if}
		<svg class="board" width="320" height="640" xmlns="http://www.w3.org/2000/svg"
			on:mousedown={boardMouseDown}
			on:mouseup={boardMouseUp}
			on:mousemove={boardMouseMove}
			on:touchstart={boardMouseDown}
			on:touchend={boardMouseUp}
			on:touchmove={boardTouchMove}
			>
			{#each paths as path}
				{#each path as point, index}
					{#if index !== 0}
						<path d={`M ${path[index - 1].x} ${path[index - 1].y} L ${point.x} ${point.y} Z`} stroke="black" stroke-width="4"/>
					{/if}
					<circle cx={point.x} cy={point.y} r="2" fill="black"/>
				{/each}
			{/each}
		</svg>
	{:else}
		{#if createRoomFailed}
		<p class="error">Uh-oh, something went wrong creating a new game, please try again!</p>
		{/if}
		<button on:click={createRoom}>
			Create a Game
		</button>
		<p>or</p>
		<input bind:value={inputRoom} />
		<button on:click={joinInputRoom}>
			Join a Game
		</button>
		{#if roomNotFound}
		<p class="error">Uh-oh, I couldn't find that room. Please try again!</p>
		{/if}
	{/if}
</main>

<style>
	main {
		--color-red: #ff3e00;

		text-align: center;
		margin: 0 auto;
	}

	h1 {
		color: var(--color-red);
		text-transform: uppercase;
		font-size: 2em;
		font-weight: 100;
		margin: 0px 0px 0.2em 0px;
	}

	p {
		margin: 0px 0px 0.7em;
	}

	.current-word {
		font-size: 2.4em;
	}

	.board {
		width: 320px;
		height: 640px;
		border: 1px solid black;
		margin: 0px auto;
		position: relative;
		display: block;
	}

	.room {
		text-transform: uppercase;
		font-weight: 600;
	}

	.error {
		color: var(--color-red);
	}

	.row {
		display: flex;
		flex-direction: row;
	}

	.space-between { justify-content: space-between; }

	@media (min-width: 640px) {
		main {
			max-width: none;
		}
	}
</style>
