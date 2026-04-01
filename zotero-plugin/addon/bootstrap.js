/* eslint-disable no-undef */
// Zotero 7/8 Bootstrap extension lifecycle
// Based on https://github.com/zotero/make-it-red and zotero-plugin-template

var chromeHandle;

function install(data, reason) {}

async function startup({ id, version, resourceURI, rootURI }, reason) {
  // Register chrome content URL so chrome://zoterorag/content/ resolves
  var aomStartup = Components.classes[
    "@mozilla.org/addons/addon-manager-startup;1"
  ].getService(Components.interfaces.amIAddonManagerStartup);
  var manifestURI = Services.io.newURI(rootURI + "manifest.json");
  chromeHandle = aomStartup.registerChrome(manifestURI, [
    ["content", "zoterorag", rootURI + "content/"],
  ]);

  await Zotero.initializationPromise;

  // Load the main plugin script
  Services.scriptloader.loadSubScript(rootURI + "content/zoterorag.js");

  // Initialize the plugin
  ZoteroRAG.init({ id, version, rootURI });
  ZoteroRAG.addToAllWindows();
}

function shutdown({ id, version, resourceURI, rootURI }, reason) {
  if (reason === APP_SHUTDOWN) return;

  ZoteroRAG.removeFromAllWindows();
  ZoteroRAG.shutdown();

  if (chromeHandle) {
    chromeHandle.destruct();
    chromeHandle = null;
  }
}

function onMainWindowLoad({ window }) {
  ZoteroRAG.addToWindow(window);
}

function onMainWindowUnload({ window }) {
  ZoteroRAG.removeFromWindow(window);
}

function uninstall(data, reason) {}
