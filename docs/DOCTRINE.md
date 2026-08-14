# Classical doctrine contract

Meridian deliberately has no switch that admits a modern planet, rulership, or
aspect. Its public `Planet` enum contains Sun, Moon, Mercury, Venus, Mars,
Jupiter, and Saturn and is exhaustively matched throughout the code.

## Chart condition

- Tropical zodiac and the five Ptolemaic aspects
- Day/night sect from the Sun's astronomical altitude
- Domicile, exaltation, Dorothean triplicity, Egyptian terms, Chaldean faces,
  detriment, and fall
- Angularity, planetary joys, sect, direct/retrograde motion, speed, cazimi,
  combustion, and under-the-beams condition
- Lot of Fortune, Spirit, Eros, Necessity, Courage, Victory, and Nemesis with
  day/night reversals
- Antiscia, contra-antiscia, and dodecatemoria for every lot

## Inspector terminology

The chart inspector identifies houses with the common medieval Latin sequence
`Vita`, `Lucrum`, `Fratres`, `Genitor`, `Nati`, `Valetudo`, `Uxor`, `Mors`,
`Iter`, `Regnum`, `Benefacta`, and `Carcer`. These are house mottos and do not
imply a modern sign-to-house correspondence.

Elemental properties use the traditional primary qualities: fire is hot and
dry, air hot and moist, water cold and moist, and earth cold and dry. Their
temperaments are respectively choleric, sanguine, phlegmatic, and melancholic.
Planetary qualities shown in the inspector are inherent baseline qualities;
they are not a computed natal temperament and do not override sign, phase,
sect, or aspect conditions.

## Aspect policy

The chart reports conjunction, sextile, square, trine, and opposition. It uses
the shortest circular separation, including across 0° Aries, and chooses the
nearest exact aspect inside the configured orb. Default base orbs are 8° for
conjunction and opposition, 7° for square and trine, and 5° for sextile. A
Sun-or-Moon aspect receives a 2° luminary allowance; aspects to the Ascendant
or Midheaven are capped at 5°, and aspects to Fortune or Spirit at 3°.

Applying and separating are determined from the instantaneous relative
longitudinal motion toward the relevant directed aspect branch. This handles
direct and retrograde motion without advancing by a fixed time step or
overshooting an imminent exact hit. An aspect is partile below 1° of orb.
Aspects to an angle or calculated lot are labelled static because the chart
does not assign those derived points an independent longitudinal speed.

## Timing and comparison

Exact transit and return times are roots solved against ephemeris longitude,
not linear estimates. Transit targets include the seven natal planets,
Ascendant, Midheaven, Fortune, and Spirit. Symbolic techniques expose their
key and symbolic date. Firdaria intentionally omits the lunar nodes so its
chronocrators remain inside the requested septenary.

Synastry compares only inter-chart septenary aspects and adds house overlays
and domicile receptions. Composite longitudes use the short circular midpoint;
Davison charts use the midpoint instant and spherical geographic midpoint and
are then recalculated as ordinary ephemeris charts.

Election scores are not presented as an oracle. The ranking records condition
of the Ascendant ruler, topical ruler, natural significator and Moon, applying
lunar aspects, angular benefics/malefics, via combusta, and fixed rising signs.
Every component and caution remains visible in the response.
