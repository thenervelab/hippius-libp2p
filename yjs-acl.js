// Y.js Access Control Layer implementation
class YjsACL {
    constructor(ydoc) {
        this.ydoc = ydoc;
        this.roles = new Y.Map();
        this.permissions = new Y.Map();
        this.signatures = new Y.Map();
        
        // Initialize ACL structure
        if (!this.roles.has('admin')) {
            this.roles.set('admin', new Y.Array());
            this.roles.set('editor', new Y.Array());
            this.roles.set('viewer', new Y.Array());
        }

        // Set up permission levels
        this.permissions.set('admin', ['read', 'write', 'manage']);
        this.permissions.set('editor', ['read', 'write']);
        this.permissions.set('viewer', ['read']);

        // Set up update handler
        this.ydoc.on('beforeTransaction', (transaction) => {
            if (!this.verifyUpdate(transaction)) {
                throw new Error('Unauthorized update');
            }
        });
    }

    // Generate a key pair for a user
    async generateKeyPair() {
        const keyPair = await window.crypto.subtle.generateKey(
            {
                name: "ECDSA",
                namedCurve: "P-256"
            },
            true,
            ["sign", "verify"]
        );
        return keyPair;
    }

    // Sign an update with user's private key
    async signUpdate(update, privateKey) {
        const signature = await window.crypto.subtle.sign(
            {
                name: "ECDSA",
                hash: { name: "SHA-256" },
            },
            privateKey,
            update
        );
        return new Uint8Array(signature);
    }

    // Verify an update's signature
    async verifySignature(update, signature, publicKey) {
        try {
            return await window.crypto.subtle.verify(
                {
                    name: "ECDSA",
                    hash: { name: "SHA-256" },
                },
                publicKey,
                signature,
                update
            );
        } catch (e) {
            console.error('Signature verification failed:', e);
            return false;
        }
    }

    // Add a user to a role
    addUserToRole(userId, role, publicKey) {
        if (!this.hasPermission(this.getCurrentUser(), 'manage')) {
            throw new Error('Unauthorized to manage roles');
        }
        
        const roleArray = this.roles.get(role);
        if (roleArray) {
            roleArray.push({ userId, publicKey });
        }
    }

    // Remove a user from a role
    removeUserFromRole(userId, role) {
        if (!this.hasPermission(this.getCurrentUser(), 'manage')) {
            throw new Error('Unauthorized to manage roles');
        }
        
        const roleArray = this.roles.get(role);
        if (roleArray) {
            const index = roleArray.findIndex(user => user.userId === userId);
            if (index !== -1) {
                roleArray.delete(index);
            }
        }
    }

    // Check if a user has a specific permission
    hasPermission(userId, permission) {
        for (const [role, users] of this.roles.entries()) {
            if (users.toArray().some(user => user.userId === userId)) {
                const rolePermissions = this.permissions.get(role);
                return rolePermissions.includes(permission);
            }
        }
        return false;
    }

    // Get user's public key
    getUserPublicKey(userId) {
        for (const users of this.roles.values()) {
            const user = users.toArray().find(u => u.userId === userId);
            if (user) {
                return user.publicKey;
            }
        }
        return null;
    }

    // Verify an update before applying
    async verifyUpdate(transaction) {
        const update = Y.encodeStateAsUpdate(this.ydoc);
        const userId = transaction.origin?.userId;
        const signature = this.signatures.get(transaction.id);

        if (!userId || !signature) {
            return false;
        }

        // Check permissions
        if (!this.hasPermission(userId, 'write')) {
            return false;
        }

        // Verify signature
        const publicKey = this.getUserPublicKey(userId);
        if (!publicKey) {
            return false;
        }

        return await this.verifySignature(update, signature, publicKey);
    }

    // Create a signed update
    async createSignedUpdate(update, userId, privateKey) {
        if (!this.hasPermission(userId, 'write')) {
            throw new Error('Unauthorized to create updates');
        }

        const signature = await this.signUpdate(update, privateKey);
        const transactionId = Math.random().toString(36).substr(2, 9);
        this.signatures.set(transactionId, signature);

        return {
            update,
            signature,
            transactionId
        };
    }

    // Initialize a new user with keys and role
    async initializeUser(userId, role = 'viewer') {
        const keyPair = await this.generateKeyPair();
        const publicKey = await window.crypto.subtle.exportKey(
            "spki",
            keyPair.publicKey
        );
        
        this.addUserToRole(userId, role, publicKey);
        
        return {
            publicKey,
            privateKey: keyPair.privateKey
        };
    }

    // Get current user from Y.js awareness
    getCurrentUser() {
        return this.ydoc.clientID;
    }
}

// Example usage:
/*
const ydoc = new Y.Doc();
const acl = new YjsACL(ydoc);

// Initialize a new admin user
const adminKeys = await acl.initializeUser('admin1', 'admin');

// Create a signed update
const update = Y.encodeStateAsUpdate(ydoc);
const signedUpdate = await acl.createSignedUpdate(update, 'admin1', adminKeys.privateKey);

// Verify update
const isValid = await acl.verifyUpdate(signedUpdate);
*/

export default YjsACL;
