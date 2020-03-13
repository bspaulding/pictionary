<script>
	let room;
	let createRoomFailed = false;
	let words = ["monkey", "banana", "motorcycle", "mountain"];
	let currentWord = words[0];

	async function createRoom() {
		const response = await fetch("/api/v1/rooms", { method: "POST" });
		if (response.ok) {
			const json = await response.json();
			room = json.room;
		} else {
			createRoomFailed = true;
		}
	}

	let isDrawing = false;
	let paths = [[]];
	function boardMouseDown(e) {
		e.preventDefault();
		e.stopImmediatePropagation();
		isDrawing = true;
	}
	function boardMouseUp(e) {
		e.preventDefault();
		e.stopImmediatePropagation();
		isDrawing = false;
		paths = [...paths, []];
	}
	function boardMouseMove(e) {
		e.preventDefault();
		e.stopImmediatePropagation();
		if (isDrawing) {
			paths = [...paths.slice(0, paths.length - 1), [...paths[paths.length - 1], { x: e.offsetX, y: e.offsetY }]];
		}
	}
</script>

<main>
	<h1>Pictionary</h1>
	{#if room}
		<p>You are in game {room}</p>
		<p>The word is: {currentWord}</p>
		<p>isDrawing: {isDrawing}</p>
		<svg class="board" width="320" height="640" xmlns="http://www.w3.org/2000/svg" on:mousedown={boardMouseDown} on:mouseup={boardMouseUp} on:mousemove={boardMouseMove}>
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
			Create Game
		</button>
	{/if}
</main>

<style>
	main {
		text-align: center;
		padding: 1em;
		max-width: 240px;
		margin: 0 auto;
	}

	h1 {
		color: #ff3e00;
		text-transform: uppercase;
		font-size: 4em;
		font-weight: 100;
	}

	.board {
		width: 320px;
		height: 640px;
		border: 1px solid black;
		margin: 0px auto;
		position: relative;
		display: block;
	}

	@media (min-width: 640px) {
		main {
			max-width: none;
		}
	}
</style>
