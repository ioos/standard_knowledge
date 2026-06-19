import init, { StandardsLibrary } from "./pkg/standard_knowledge_js.js";

// Need to initialize WASM before we can use the library
await init();

class Standard extends HTMLElement {
	#standard;
	#show = false;

	set standard(standard) {
		this.#standard = standard;
		this.update();
	}

	update() {
		const attrs = Array.from(this.#standard.attrs());

		const mapToObj = (m) => {
			return Array.from(m).reduce((obj, [key, value]) => {
				obj[key] = value;
				return obj;
			}, {});
		};

		this.innerHTML = `
            <div class="card">
                <div class="card-header bg-primary-subtle">
                    <h4>
                        ${this.#standard.name}
                    </h4>
                </div>
                <div class="card-body">
                    <table class="table">
                        <tr class="table-primary">
                            <td>Unit</td>
                            <td>${this.#standard.unit}</td>
                        </tr>
                        <tr class="table-primary">
                            <td>Aliases</td>
                            <td>${this.#standard.aliases.join(", ")}</td>
                        </tr>
                        <tr title="A more human readable name for the standard">
                            <td>Long Name</td>
                            <td>${this.#standard.longName}</td>
                        </tr>
                        <tr title="Category of measurement for the Integrated Ocean Observing System">
                            <td>IOOS Category</td>
                            <td>${this.#standard.ioosCategory}</td>
                        </tr>
                        <tr title="When the standard name isn't used for a column or variable, what might commonly get used instead.">
                            <td>Common variable names</td>
                            <td>${this.#standard.commonVariableNames.join(", ")}</td>
                        </tr>
                        <tr title="Standards that are usually used together">
                            <td>Sibling Standards</td>
                            <td>${this.#standard.siblingStandards.join(", ")}</td>
                        </tr>
                        <tr title="Standards that measure generally similar things, but differ in specifics that are worth investigating.">
                            <td>Related standards</td>
                            <td>${this.#standard.relatedStandards.join(", ")}</td>
                        </tr>
                        <tr title="Other units that may be used rather than the one defined in the standard.">
                            <td>Other Units</td>
                            <td>${this.#standard.otherUnits.join(", ")}</td>
                        </tr>
                    </table>

                    <div class="bg-primary-subtle">
                    <h4>Description:</h4>
                    <p>${this.#standard.description}</p>
                    </div>

                    ${
											attrs.length === 0
												? "<p>No attributes</p>"
												: `
                        <h4>Suggested attributes:</h4>

                        <details><summary>As JSON</summary>
                        <code>
                            ${JSON.stringify(mapToObj(this.#standard.attrs()), null, 2)}
                        </code>
                        </details>

                        <details><summary>As YAML</summary>

                        <pre><code>
${attrs.map((a) => `${a[0]}: ${a[1]}`).join("\n")}
                        </code></pre>
                        </details>

                        <details><summary>For ERDDAP</summary>
                        <pre><code>
${attrs.map((a) => `&lt;att name="${a[0]}"&gt;${a[1]}&lt;/att&gt;`).join("\n")}
                        </code></pre>
                        </details>
                    `
										}

                    ${
											this.#standard.comments
												? `
                        <h4>Comments:</h4>
                        <p>${this.#standard.comments}</p>
                    `
												: ""
										}

                    ${
											this.#standard.qartod.length > 0
												? `
                        <h4>QARTOD Tests:</h4>
                        <ul>
                            ${this.#standard.qartod.map((q) => `<li>${q.name} (<code>${q.slug}</code>) <p>${q.description}</p></li>`).join("")}
                        </ul>
                    `
												: ""
										}
                </div>
            </div>
        `;
	}
}

customElements.define("x-standard", Standard);

class FilterStandards extends HTMLElement {
	#library;
	varName = "";
	ioosCategory = "";
	unit = "";
	qartodTests = false;
	search = "";

	set library(newLibrary) {
		this.#library = newLibrary;

		this.querySelector("#ioosCategory").outerHTML = `
            <select id="ioosCategory" name="ioosCategory">
                <option value="">Select IOOS Category</option>
                ${this.#library
									.knownIoosCategories()
									.map(
										(cat) => `
                    <option value="${cat}">${cat}</option>
                `,
									)
									.join("")}
            </select>
        `;

		this.querySelector("#ioosCategory").addEventListener("input", (e) => {
			this.ioosCategory = e.target.value;
			this.update();
		});
	}

	connectedCallback() {
		this.innerHTML = `
            <div class="accordion-item">
                <div class="accordion-header">
                    <button class="accordion-button" type="button" data-bs-toggle="collapse" data-bs-target="#collapseFilter" aria-expanded="true" aria-controls="collapseFilter">
                        <h2>Filter standards</h2>
                    </button>
                </div>
                <div id="collapseFilter" class="accordion-collapse collapse show">
                    <div class="accordion-body">
                        <div class="mb-3">
                            <label for="varName">By common variable names</label>
                            <input type="text" id="varName" name="varName" placeholder="Filter by common variable names" />
                        </div>

                        <div class="mb-3">
                            <label for="ioosCategory">By IOOS Category</label>
                            <select id="ioosCategory" name="ioosCategory">
                                <option value="">Select IOOS Category</option>
                            </select>
                        </div>

                        <div class="mb-3">
                            <label for="unit">By Unit</label>
                            <input type="text" id="unit" name="unit" placeholder="Filter by Unit" />
                        </div>

                        <div class="mb-3">
                            <label for="qartodTests">By QARTOD Tests</label>
                            <input type="checkbox" id="qartodTests" name="qartodTests" />
                        </div>

                        <div class="mb-3">
                            <label for="search">Text Search</label>
                            <input type="text" id="search" name="search" placeholder="Filter by Search" />
                        </div>

                        <div id="filterResult">
                            Please enter a keyword
                        </div>
                    </div>
                </div>
            </div>
        `;

		this.querySelector("#varName").addEventListener("input", (e) => {
			this.varName = e.target.value;
			this.update();
		});

		this.querySelector("#unit").addEventListener("input", (e) => {
			this.unit = e.target.value;
			this.update();
		});

		this.querySelector("#qartodTests").addEventListener("change", (e) => {
			this.qartodTests = e.target.checked;
			this.update();
		});

		this.querySelector("#search").addEventListener("input", (e) => {
			this.search = e.target.value;
			this.update();
		});
	}

	update() {
		let filter = this.#library.filter();

		if (this.varName) {
			filter = filter.byVariableName(this.varName);
		}

		if (this.ioosCategory) {
			filter = filter.byIoosCategory(this.ioosCategory);
		}

		if (this.unit) {
			filter = filter.byUnit(this.unit);
		}

		if (this.qartodTests) {
			filter = filter.hasQartodTests(this.qartodTests);
		}

		if (this.search) {
			filter = filter.search(this.search);
		}

		const standards = filter.standards;

		if (standards.length === this.#library.filter().standards.length) {
			this.querySelector("#filterResult").innerHTML = `
                <div class="alert alert-info" role="alert">
                    Please enter filters
                </div>
            `;
		} else if (standards.length === 0) {
			this.querySelector("#filterResult").innerHTML = `
                <div class="alert alert-warning" role="alert">
                    No standards found
                </div>
            `;
		} else {
			this.querySelector("#filterResult").innerHTML = `
                <ul>
                    ${standards
											.map((s) => {
												if (s.longName) {
													return `<li>${s.longName} - <span class="bg-primary-subtle">${s.name}</span></li>`;
												}

												return `<li><span class="bg-primary-subtle">${s.name}</span></li>`;
											})
											.join("")}
                </ul>
            `;
		}
	}
}

customElements.define("x-filter-standards", FilterStandards);

class GetStandard extends HTMLElement {
	connectedCallback() {
		this.innerHTML = `
        <div class="accordion-item">
            <div class="accordion-header">
                <button class="accordion-button" type="button" data-bs-toggle="collapse" data-bs-target="#collapseGet" aria-expanded="true" aria-controls="collapseGet">
                    <h2>Get knowledge about a standard</h2>
                </button>
            </div>
            <div id="collapseGet" class="accordion-collapse collapse show">
                <div class="accordion-body">
                    <form>
                        <input type="text" id="name" name="name" placeholder="Enter a CF standard name" />
                        <button type="submit">Show Standard</button>
                    </form>
                    <div id="result">
                        Please enter a standard
                    </div>
                </div>
            </div>
        </div>
        `;

		this.querySelector("form").onsubmit = (e) => {
			e.preventDefault();
			const data = new FormData(e.target);
			const name = data.get("name").toString().trim();

			if (name) {
				try {
					const standard = this.library.get(name); // Just to see if it exists
					this.querySelector("#result").innerHTML = `
                        <x-standard></x-standard>
                    `;
					this.querySelector("x-standard").standard = standard;
				} catch (error) {
					this.querySelector("#result").innerHTML = `
                    <div class="alert alert-danger" role="alert">
                        Could not find standard with name <strong>${name}</strong>
                    </div>
                    `;
					console.error(error);
				}
			}
		};
	}
}

customElements.define("x-get-standard", GetStandard);

class App extends HTMLElement {
	connectedCallback() {
		this.textContent = "Invalid standard";

		this.library = new StandardsLibrary();
		this.library.loadCfStandards();
		this.library.loadKnowledge();
		this.library.loadTestSuites();

		this.innerHTML = `
        <div class="accordion">
            <x-filter-standards></x-filter-standards>
            <x-get-standard></x-get-standard>
        </div>
        `;

		this.querySelector("x-get-standard").library = this.library;
		this.querySelector("x-filter-standards").library = this.library;
	}
}

customElements.define("x-app", App);

// const app = () => {
//     customElements.define("x-standard", Standard);
//     customElements.define("x-app", App);
// }

// document.addEventListener("DOMContentLoaded", app);
