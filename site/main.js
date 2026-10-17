(function () {
  "use strict";

  var REPO = "azmisahin-gov/forensic-transcriber-next";
  var RELEASES = "https://github.com/" + REPO + "/releases";
  var API = "https://api.github.com/repos/" + REPO + "/releases/latest";

  // Language toggle.
  var buttons = document.querySelectorAll(".lang button");
  function setLang(lang) {
    document.documentElement.setAttribute("lang", lang);
    buttons.forEach(function (b) {
      b.classList.toggle("active", b.getAttribute("data-lang") === lang);
    });
    document.querySelectorAll("[data-tr][data-en]").forEach(function (el) {
      var value = el.getAttribute("data-" + lang);
      if (value !== null) el.textContent = value;
    });
    try {
      localStorage.setItem("ftn-lang", lang);
    } catch (e) {}
  }
  buttons.forEach(function (b) {
    b.addEventListener("click", function () {
      setLang(b.getAttribute("data-lang"));
    });
  });
  var saved = "tr";
  try {
    saved = localStorage.getItem("ftn-lang") || "tr";
  } catch (e) {}
  setLang(saved === "en" ? "en" : "tr");

  // Latest release (dynamic; falls back to the releases page).
  var versionEl = document.getElementById("latest-version");
  var sourceEl = document.getElementById("version-source");
  var downloadEl = document.getElementById("download-primary");

  fetch(API, { headers: { Accept: "application/vnd.github+json" } })
    .then(function (r) {
      if (!r.ok) throw new Error("no release");
      return r.json();
    })
    .then(function (release) {
      var tag = release.tag_name || "";
      var version = tag.replace(/^v/, "");
      versionEl.textContent = version;
      var assetName = "ForensicTranscriberNext_" + version + "_x64-setup.exe";
      downloadEl.href = RELEASES + "/download/" + tag + "/" + assetName;
    })
    .catch(function () {
      // No release published yet: keep the link pointing at the Releases page.
      versionEl.textContent = "—";
      if (sourceEl) sourceEl.textContent = "(henüz yayımlanmadı / not published yet)";
      downloadEl.href = RELEASES;
    });
})();
