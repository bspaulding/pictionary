import App from './App.svelte';

Sentry.init({ dsn: 'https://50ca82c37131420b9e5deead3ba0695a@sentry.io/4977551' });

const app = new App({
	target: document.body,
	props: {}
});

export default app;
