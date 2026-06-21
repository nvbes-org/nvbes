export function optionValue(args, flag, fallback = undefined) {
	const index = args.indexOf(flag);
	if (index >= 0) return args[index + 1] ?? fallback;

	const prefix = `${flag}=`;
	const inline = args.find((arg) => arg.startsWith(prefix));
	return inline ? inline.slice(prefix.length) : fallback;
}
