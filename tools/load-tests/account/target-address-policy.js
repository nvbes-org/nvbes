const IPV4_NON_PUBLIC_NETWORKS = [
	["0.0.0.0", 8],
	["10.0.0.0", 8],
	["100.64.0.0", 10],
	["127.0.0.0", 8],
	["169.254.0.0", 16],
	["172.16.0.0", 12],
	["192.0.0.0", 24],
	["192.0.2.0", 24],
	["192.88.99.0", 24],
	["192.168.0.0", 16],
	["198.18.0.0", 15],
	["198.51.100.0", 24],
	["203.0.113.0", 24],
	["224.0.0.0", 4],
	["240.0.0.0", 4],
];

const IPV6_NON_PUBLIC_NETWORKS = [
	["2001:0::", 23],
	["2001:db8::", 32],
	["2002::", 16],
	["3fff::", 20],
];

export function assertPublicNetworkAddress(address) {
	const family = ipAddressFamily(address);
	if (family === 4 && isPublicIpv4(address)) {
		return;
	}
	if (family === 6 && isPublicIpv6(address)) {
		return;
	}
	throw new Error(
		`Target DNS resolved to a non-public or reserved address: ${address}`,
	);
}

export function ipAddressFamily(address) {
	if (parseIpv4(address) !== null) {
		return 4;
	}
	if (parseIpv6(address) !== null) {
		return 6;
	}
	return 0;
}

function isPublicIpv4(address) {
	const value = parseIpv4(address);
	return (
		value !== null &&
		!IPV4_NON_PUBLIC_NETWORKS.some(([network, prefix]) =>
			matchesPrefix(value, parseIpv4(network), prefix, 32),
		)
	);
}

function isPublicIpv6(address) {
	const value = parseIpv6(address);
	if (value === null) {
		return false;
	}

	const mappedIpv4Prefix = 0xffffn;
	if (value >> 32n === mappedIpv4Prefix) {
		return isPublicIpv4(renderIpv4(Number(value & 0xffff_ffffn)));
	}

	const globalUnicast = matchesPrefix(value, parseIpv6("2000::"), 3, 128);
	return (
		globalUnicast &&
		!IPV6_NON_PUBLIC_NETWORKS.some(([network, prefix]) =>
			matchesPrefix(value, parseIpv6(network), prefix, 128),
		)
	);
}

function parseIpv4(address) {
	if (typeof address !== "string" || !/^\d+\.\d+\.\d+\.\d+$/u.test(address)) {
		return null;
	}
	const octets = address.split(".").map(Number);
	if (octets.some((octet) => !Number.isSafeInteger(octet) || octet > 255)) {
		return null;
	}
	return octets.reduce((value, octet) => value * 256 + octet, 0);
}

function parseIpv6(address) {
	if (
		typeof address !== "string" ||
		address === "" ||
		address.includes("%") ||
		(address.match(/::/gu) || []).length > 1
	) {
		return null;
	}

	const converted = convertIpv4Suffix(address.toLowerCase());
	if (converted === null) {
		return null;
	}
	const hasCompression = converted.includes("::");
	const [leftRaw, rightRaw = ""] = converted.split("::");
	const left = leftRaw === "" ? [] : leftRaw.split(":");
	const right = rightRaw === "" ? [] : rightRaw.split(":");
	const missing = 8 - left.length - right.length;
	if ((!hasCompression && missing !== 0) || (hasCompression && missing < 1)) {
		return null;
	}

	const groups = [
		...left,
		...Array.from({ length: missing }, () => "0"),
		...right,
	];
	if (
		groups.length !== 8 ||
		groups.some((group) => !/^[0-9a-f]{1,4}$/u.test(group))
	) {
		return null;
	}
	return groups.reduce(
		(value, group) => (value << 16n) | BigInt(`0x${group}`),
		0n,
	);
}

function convertIpv4Suffix(address) {
	if (!address.includes(".")) {
		return address;
	}
	const separator = address.lastIndexOf(":");
	if (separator < 0) {
		return null;
	}
	const ipv4 = parseIpv4(address.slice(separator + 1));
	if (ipv4 === null) {
		return null;
	}
	const high = ((ipv4 >>> 16) & 0xffff).toString(16);
	const low = (ipv4 & 0xffff).toString(16);
	return `${address.slice(0, separator)}:${high}:${low}`;
}

function matchesPrefix(value, network, prefix, width) {
	if (value === null || network === null) {
		return false;
	}
	const shift = BigInt(width - prefix);
	return BigInt(value) >> shift === BigInt(network) >> shift;
}

function renderIpv4(value) {
	return [24, 16, 8, 0].map((shift) => (value >>> shift) & 0xff).join(".");
}
